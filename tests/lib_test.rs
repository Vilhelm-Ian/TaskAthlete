// tests/lib_test.rs
use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::thread;
use std::time::Duration as StdDuration;
use task_athlete_lib::{
    AddWorkoutParams, AppService, Config, ConfigError, DbError, EditWorkoutParams, ExerciseType,
    GraphType, Units, VolumeFilters, WorkoutFilters,
};

// Helper function to create a test service with in-memory database
fn create_test_service() -> Result<AppService> {
    let conn = rusqlite::Connection::open_in_memory()?;
    task_athlete_lib::db::init(&conn)?; // Use renamed init

    // Create a default config for testing
    let mut config = Config::default();
    config.bodyweight = Some(70.0); // Set default bodyweight
    config.units = Units::Metric;
    config.pb_notifications.enabled = Some(true); // Enable PB checks by default for tests
    config.pb_notifications.notify_weight = true;
    config.pb_notifications.notify_reps = true;
    config.pb_notifications.notify_duration = true;
    config.pb_notifications.notify_distance = true;

    Ok(AppService {
        config,
        conn,
        db_path: ":memory:".into(),
        config_path: "test_config.toml".into(),
    })
}

#[test]
fn test_create_exercise_unique_name() -> Result<()> {
    let service = create_test_service()?;
    service.create_exercise("Bench Press", ExerciseType::Resistance, None, Some("chest"))?;

    // Try creating with same name (case-insensitive)
    let result = service.create_exercise("bench press", ExerciseType::Cardio, None, None);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Exercise name must be unique"));
        // Optionally check the underlying DbError type
        assert!(matches!(
            e.downcast_ref::<DbError>(),
            Some(DbError::ExerciseNameNotUnique(_))
        ));
    }

    // Try creating with different name
    let result = service.create_exercise("Squat", ExerciseType::Resistance, None, Some("legs"));
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_exercise_aliases() -> Result<()> {
    let mut service = create_test_service()?;

    let ex_id = service.create_exercise(
        "Barbell Bench Press",
        ExerciseType::Resistance,
        None,
        Some("chest"),
    )?;
    service.create_exercise("Squat", ExerciseType::Resistance, None, Some("Legs"))?;

    // 1. Create Alias
    service.create_alias("bp", "Barbell Bench Press")?;

    // 2. List Aliases
    let aliases = service.list_aliases()?;
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases.get("bp"), Some(&"Barbell Bench Press".to_string()));

    // 3. Resolve Alias
    let resolved_def = service.resolve_exercise_identifier("bp")?;
    assert!(resolved_def.is_some());
    let resolved_def = resolved_def.unwrap(); // Safe unwrap
    assert_eq!(resolved_def.name, "Barbell Bench Press");
    assert_eq!(resolved_def.id, ex_id);

    // 4. Try creating duplicate alias
    let result = service.create_alias("bp", "Squat"); // Different exercise, same alias
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Alias already exists"));

    // 5. Try creating alias conflicting with name/id
    let result = service.create_alias("Barbell Bench Press", "Squat"); // Alias conflicts with name
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("conflicts with an existing exercise name"));

    let result = service.create_alias(&ex_id.to_string(), "Squat"); // Alias conflicts with ID
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("conflicts with an existing exercise ID"));

    // 6. Use Alias in Add Workout
    let today = Utc::now();
    let add_params = AddWorkoutParams {
        exercise_identifier: "bp",
        date: today,
        sets: Some(3),
        reps: Some(5),
        weight: Some(100.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    };
    let (workout_id, _) = service.add_workout(add_params)?;

    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("bp"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, workout_id);
    assert_eq!(workouts[0].exercise_name, "Barbell Bench Press"); // Stored canonical name

    // 7. Delete Alias
    let deleted_count = service.delete_alias("bp")?;
    assert_eq!(deleted_count, 1);
    let aliases_after_delete = service.list_aliases()?;
    assert!(aliases_after_delete.is_empty());

    // 8. Try deleting non-existent alias
    let result = service.delete_alias("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Alias not found"));

    Ok(())
}

#[test]
fn test_edit_exercise_with_alias_and_name_change() -> Result<()> {
    let mut service = create_test_service()?;

    service.create_exercise("Old Name", ExerciseType::Resistance, None, Some("muscle1"))?;
    service.create_alias("on", "Old Name")?;

    // Add a workout using the alias
    let today = Utc::now();
    let add_params = AddWorkoutParams {
        exercise_identifier: "on",
        date: today,
        sets: Some(1),
        reps: Some(1),
        weight: Some(1.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    };
    service.add_workout(add_params)?;

    // Edit using the alias identifier
    service.edit_exercise(
        "on", // Identify by alias
        Some("New Name"),
        None,
        None,                          // Keep type
        Some(Some("muscle1,muscle2")), // Change muscles
    )?;

    // --- Verification ---

    // 1. Old name resolves to None
    assert!(service.resolve_exercise_identifier("Old Name")?.is_none());

    // 2. Alias resolves to new definition
    let resolved_by_alias = service
        .resolve_exercise_identifier("on")?
        .expect("Alias 'on' should resolve");
    assert_eq!(resolved_by_alias.name, "New Name");
    assert_eq!(
        resolved_by_alias.muscles,
        Some("muscle1,muscle2".to_string())
    );

    // 3. New name resolves correctly
    let resolved_by_new_name = service
        .resolve_exercise_identifier("New Name")?
        .expect("'New Name' should resolve");
    assert_eq!(resolved_by_new_name.id, resolved_by_alias.id);
    assert_eq!(resolved_by_new_name.name, "New Name");
    assert_eq!(
        resolved_by_new_name.muscles,
        Some("muscle1,muscle2".to_string())
    );

    // 4. Alias list points to NEW name
    let aliases = service.list_aliases()?;
    assert_eq!(
        aliases.get("on").expect("Alias 'on' should exist"),
        "New Name"
    );

    // 5. Workout entry updated
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("on"), // List using alias
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1, "Should find one workout via alias");
    assert_eq!(workouts[0].exercise_name, "New Name");

    // List workouts using old name (should fail or be empty)
    let workouts_old_name = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Old Name"),
        ..Default::default()
    });
    assert!(workouts_old_name.is_err()); // Expect ExerciseNotFound error
    assert!(matches!(
        workouts_old_name.unwrap_err().downcast_ref::<DbError>(),
        Some(DbError::ExerciseNotFound(_))
    ));

    Ok(())
}

#[test]
fn test_delete_exercise_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("To Delete", ExerciseType::Cardio, None, None)?;
    service.create_alias("td", "To Delete")?;

    // Delete exercise using alias
    let result = service.delete_exercise(&["td".to_string()])?;
    assert_eq!(result, 1);

    // Verify exercise is gone
    assert!(service.resolve_exercise_identifier("To Delete")?.is_none());
    assert!(service.resolve_exercise_identifier("td")?.is_none());

    // Verify alias is gone
    let aliases = service.list_aliases()?;
    assert!(aliases.is_empty());

    Ok(())
}

#[test]
fn test_add_workout_past_date() -> Result<()> {
    let mut service = create_test_service()?;
    let res = service.list_exercises(None, None);
    println!("here");
    res.unwrap_or(Vec::new())
        .into_iter()
        .for_each(|a| println!("{}", a.name));
    service.create_exercise("Rowing", ExerciseType::Cardio, None, None)?;

    let yesterday = Utc::now() - Duration::days(1);
    let two_days_ago = Utc::now() - Duration::days(2);

    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Rowing",
        date: yesterday,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(30),
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;
    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Rowing",
        date: two_days_ago,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(25),
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;

    // List for yesterday
    let workouts_yesterday = service.list_workouts(&WorkoutFilters {
        date: Some(yesterday.date_naive()),
        ..Default::default()
    })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].duration_minutes, Some(30));
    assert_eq!(
        workouts_yesterday[0].timestamp.date_naive(),
        yesterday.date_naive()
    );

    // List for two days ago
    let workouts_two_days_ago = service.list_workouts(&WorkoutFilters {
        date: Some(two_days_ago.date_naive()),
        ..Default::default()
    })?;
    assert_eq!(workouts_two_days_ago.len(), 1);
    assert_eq!(workouts_two_days_ago[0].duration_minutes, Some(25));
    assert_eq!(
        workouts_two_days_ago[0].timestamp.date_naive(),
        two_days_ago.date_naive()
    );

    Ok(())
}

#[test]
fn test_edit_workout_date() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Push-ups", ExerciseType::BodyWeight, None, None)?;
    let today = Utc::now();
    let yesterday = today - Duration::days(1);

    let (workout_id, _) = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Push-ups",
        date: today,
        sets: Some(3),
        reps: Some(15),
        weight: None,
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: Some(70.0), // Use config bodyweight
    })?;

    // Edit the date
    let mut params = EditWorkoutParams::default();
    params.id = workout_id;
    params.new_date = Some(yesterday.date_naive());
    service.edit_workout(params)?;

    // Verify date change by listing
    let workouts_today = service.list_workouts(&WorkoutFilters {
        date: Some(today.date_naive()),
        ..Default::default()
    })?;
    assert!(workouts_today.is_empty());

    let workouts_yesterday = service.list_workouts(&WorkoutFilters {
        date: Some(yesterday.date_naive()),
        ..Default::default()
    })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].id, workout_id);
    assert_eq!(
        workouts_yesterday[0].timestamp.date_naive(),
        yesterday.date_naive()
    );

    Ok(())
}

// Refactored PB test using new structures and configs
#[test]
fn test_pb_detection_and_config() -> Result<()> {
    let mut service = create_test_service()?; // Uses test default config (all PBs enabled)
    service.create_exercise(
        "Deadlift",
        ExerciseType::Resistance,
        None,
        Some("back,legs"),
    )?;
    service.create_exercise("Running", ExerciseType::Cardio, None, Some("legs"))?;
    let today = Utc::now();

    // Helper macro for adding workout
    macro_rules! add_dl_workout {
        ($sets:expr, $reps:expr, $weight:expr) => {
            service.add_workout(AddWorkoutParams {
                exercise_identifier: "Deadlift",
                date: today,
                sets: $sets,
                reps: $reps,
                weight: $weight,
                ..Default::default()
            })
        };
    }
    macro_rules! add_run_workout {
        ($duration:expr, $distance:expr) => {
            service.add_workout(AddWorkoutParams {
                exercise_identifier: "Running",
                date: today,
                duration: $duration,
                distance: $distance,
                ..Default::default()
            })
        };
    }

    // --- Test Default Config (All PBs Enabled) ---

    // Workout 1: Baseline
    let (_, pb1) = add_dl_workout!(Some(1), Some(5), Some(100.0))?;
    assert!(pb1.is_none(), "PB1: First workout shouldn't be a PB");
    thread::sleep(StdDuration::from_millis(10)); // Ensure timestamp difference if needed

    // Workout 2: Weight PB
    let (_, pb2) = add_dl_workout!(Some(1), Some(3), Some(110.0))?;
    assert!(pb2.is_some(), "PB2: Should detect weight PB");
    let info2 = pb2.unwrap();
    assert!(info2.weight.achieved, "PB2: Weight PB flag");
    assert!(!info2.reps.achieved, "PB2: Reps PB flag");
    assert_eq!(info2.weight.new_value, Some(110.0), "PB2: New weight");
    assert_eq!(info2.weight.previous_value, Some(100.0), "PB2: Prev weight");
    thread::sleep(StdDuration::from_millis(10));

    // Workout 3: Reps PB
    let (_, pb3) = add_dl_workout!(Some(3), Some(6), Some(90.0))?;
    assert!(pb3.is_some(), "PB3: Should detect reps PB");
    let info3 = pb3.unwrap();
    assert!(!info3.weight.achieved, "PB3: Weight PB flag");
    assert!(info3.reps.achieved, "PB3: Reps PB flag");
    assert_eq!(info3.reps.new_value, Some(6), "PB3: New reps");
    assert_eq!(info3.reps.previous_value, Some(5), "PB3: Prev reps");
    thread::sleep(StdDuration::from_millis(10));

    // Workout 4: Both Weight and Reps PB
    let (_, pb4) = add_dl_workout!(Some(1), Some(7), Some(120.0))?;
    assert!(pb4.is_some(), "PB4: Should detect both PBs");
    let info4 = pb4.unwrap();
    assert!(info4.weight.achieved, "PB4: Weight PB flag");
    assert!(info4.reps.achieved, "PB4: Reps PB flag");
    assert_eq!(info4.weight.new_value, Some(120.0), "PB4: New weight");
    assert_eq!(info4.weight.previous_value, Some(110.0), "PB4: Prev weight");
    assert_eq!(info4.reps.new_value, Some(7), "PB4: New reps");
    assert_eq!(info4.reps.previous_value, Some(6), "PB4: Prev reps");
    thread::sleep(StdDuration::from_millis(10));

    // Workout 5: No PB
    let (_, pb5) = add_dl_workout!(Some(5), Some(5), Some(105.0))?;
    assert!(pb5.is_none(), "PB5: Should not detect PB");
    thread::sleep(StdDuration::from_millis(10));

    // --- Test Disabling Specific PBs ---
    service.set_pb_notify_reps(false)?; // Disable Rep PB notifications

    // Workout 6: Weight PB (should be detected)
    let (_, pb6) = add_dl_workout!(Some(1), Some(4), Some(130.0))?;
    assert!(pb6.is_some(), "PB6: Weight PB should still be detected");
    assert!(pb6.unwrap().weight.achieved, "PB6: Weight PB flag");
    thread::sleep(StdDuration::from_millis(10));

    // Workout 7: Reps PB (should NOT be detected as PB *notification*)
    let (_, pb7) = add_dl_workout!(Some(1), Some(8), Some(125.0))?; // 8 reps > previous max 7
    assert!(
        pb7.is_none(),
        "PB7: Reps PB should NOT trigger notification"
    );
    // Note: The *actual* max reps in DB is now 8, but no PBInfo returned.
    thread::sleep(StdDuration::from_millis(10));

    // --- Test Duration/Distance PBs ---
    service.set_pb_notify_reps(true)?; // Re-enable reps
    service.set_pb_notify_weight(false)?; // Disable weight

    // Running Workout 1: Baseline
    let (_, rpb1) = add_run_workout!(Some(30), Some(5.0))?; // 5km in 30min
    assert!(rpb1.is_none(), "RPB1: First run no PB");
    thread::sleep(StdDuration::from_millis(10));

    // Running Workout 2: Duration PB (longer duration, same distance)
    let (_, rpb2) = add_run_workout!(Some(35), Some(5.0))?;
    assert!(rpb2.is_some(), "RPB2: Should detect duration PB");
    let rinfo2 = rpb2.unwrap();
    assert!(rinfo2.duration.achieved, "RPB2: Duration flag");
    assert!(!rinfo2.distance.achieved, "RPB2: Distance flag");
    assert_eq!(rinfo2.duration.new_value, Some(35), "RPB2: New duration");
    assert_eq!(
        rinfo2.duration.previous_value,
        Some(30),
        "RPB2: Prev duration"
    );
    thread::sleep(StdDuration::from_millis(10));

    // Running Workout 3: Distance PB (longer distance, irrelevant duration)
    let (_, rpb3) = add_run_workout!(Some(25), Some(6.0))?;
    assert!(rpb3.is_some(), "RPB3: Should detect distance PB");
    let rinfo3 = rpb3.unwrap();
    assert!(!rinfo3.duration.achieved, "RPB3: Duration flag");
    assert!(rinfo3.distance.achieved, "RPB3: Distance flag");
    assert_eq!(
        rinfo3.distance.new_value,
        Some(6.0),
        "RPB3: New distance (km)"
    );
    assert_eq!(
        rinfo3.distance.previous_value,
        Some(5.0),
        "RPB3: Prev distance (km)"
    );
    thread::sleep(StdDuration::from_millis(10));

    // Running Workout 4: Disable distance PB, achieve distance PB -> No notification for distance
    service.set_pb_notify_distance(false)?;
    let (_, rpb4) = add_run_workout!(Some(40), Some(7.0))?; // Both duration and distance are PBs
    assert!(rpb4.is_some(), "RPB4: Duration PB should still trigger");
    let rinfo4 = rpb4.unwrap();
    assert!(rinfo4.duration.achieved, "RPB4: Duration flag"); // Duration PB is still active
    assert!(
        !rinfo4.distance.achieved,
        "RPB4: Distance PB flag should be false if disabled"
    );
    assert_eq!(rinfo4.duration.new_value, Some(40), "RPB4: New duration");
    assert_eq!(
        rinfo4.duration.previous_value,
        Some(35),
        "RPB4: Prev duration"
    );
    assert_eq!(
        rinfo4.distance.new_value,
        Some(7.0),
        "RPB4: New distance (still recorded)"
    );
    assert_eq!(
        rinfo4.distance.previous_value,
        Some(6.0),
        "RPB4: Prev distance"
    );

    Ok(())
}

#[test]
fn test_create_and_list_exercises() -> Result<()> {
    let service = create_test_service()?;

    // Create exercises
    service.create_exercise(
        "Bench Press",
        ExerciseType::Resistance,
        None,
        Some("chest,triceps"),
    )?;
    service.create_exercise("Running", ExerciseType::Cardio, None, Some("legs"))?;
    service.create_exercise(
        "Pull-ups",
        ExerciseType::BodyWeight,
        None,
        Some("back,biceps"),
    )?;

    // List all
    let exercises = service.list_exercises(None, None)?;
    assert_eq!(exercises.len(), 3);

    // Filter by type
    let resistance_exercises = service.list_exercises(Some(ExerciseType::Resistance), None)?;
    assert_eq!(resistance_exercises.len(), 1);
    assert_eq!(resistance_exercises[0].name, "Bench Press");

    // Filter by muscle
    let leg_exercises = service.list_exercises(None, Some("legs"))?;
    assert_eq!(leg_exercises.len(), 1); // Running
    assert_eq!(leg_exercises[0].name, "Running");

    let back_exercises = service.list_exercises(None, Some("back"))?;
    assert_eq!(back_exercises.len(), 1); // Pull-ups
    assert_eq!(back_exercises[0].name, "Pull-ups");

    Ok(())
}

#[test]
fn test_pb_config_interaction() -> Result<()> {
    let mut service = create_test_service()?; // PB enabled: Some(true) initially

    assert!(service.check_pb_notification_config()?);

    // Disable globally
    service.set_pb_notification_enabled(false)?;
    assert_eq!(service.config.pb_notifications.enabled, Some(false));
    assert!(!service.check_pb_notification_config()?);

    // Re-enable globally
    service.set_pb_notification_enabled(true)?;
    assert_eq!(service.config.pb_notifications.enabled, Some(true));
    assert!(service.check_pb_notification_config()?);

    // Test when None (simulate first run)
    service.config.pb_notifications.enabled = None;
    let result = service.check_pb_notification_config();
    assert!(result.is_err());
    assert!(matches!(result, Err(ConfigError::PbNotificationNotSet)));

    // Test individual metric flags
    service.config.pb_notifications.enabled = Some(true); // Ensure global is enabled
    assert!(service.config.pb_notifications.notify_weight);
    service.set_pb_notify_weight(false)?;
    assert!(!service.config.pb_notifications.notify_weight);
    service.set_pb_notify_weight(true)?;
    assert!(service.config.pb_notifications.notify_weight);

    assert!(service.config.pb_notifications.notify_reps);
    service.set_pb_notify_reps(false)?;
    assert!(!service.config.pb_notifications.notify_reps);

    assert!(service.config.pb_notifications.notify_duration);
    service.set_pb_notify_duration(false)?;
    assert!(!service.config.pb_notifications.notify_duration);

    assert!(service.config.pb_notifications.notify_distance);
    service.set_pb_notify_distance(false)?;
    assert!(!service.config.pb_notifications.notify_distance);

    Ok(())
}

#[test]
fn test_list_filter_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise(
        "Overhead Press",
        ExerciseType::Resistance,
        None,
        Some("shoulders"),
    )?;
    service.create_alias("ohp", "Overhead Press")?;
    let today = Utc::now();

    service.add_workout(AddWorkoutParams {
        exercise_identifier: "ohp",
        date: today,
        sets: Some(5),
        reps: Some(5),
        weight: Some(50.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;

    // Filter list using alias
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("ohp"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].exercise_name, "Overhead Press");

    // Filter list using canonical name
    let workouts2 = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Overhead Press"),
        ..Default::default()
    })?;
    assert_eq!(workouts2.len(), 1);
    assert_eq!(workouts2[0].exercise_name, "Overhead Press");

    Ok(())
}

#[test]
fn test_add_and_list_workouts_with_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Metric; // Use Metric for easy km verification

    service.create_exercise("Running", ExerciseType::Cardio, None, Some("legs"))?;
    let naive_date = NaiveDate::from_ymd_opt(2023, 6, 2).unwrap();
    let naive_datetime = naive_date.and_hms_opt(0, 0, 0).unwrap();
    let date1: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let naive_date = NaiveDate::from_ymd_opt(2023, 6, 2).unwrap();
    let naive_datetime = naive_date.and_hms_opt(0, 0, 0).unwrap();
    let date2: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Running",
        date: date1,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(30),
        distance: Some(5.0), // 5 km
        notes: Some("First run".to_string()),
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;
    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Running",
        date: date2,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(60),
        distance: Some(10.5), // 10.5 km
        notes: Some("Second run".to_string()),
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;

    // List workouts
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Running"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 2);
    // Most recent first
    assert_eq!(workouts[0].timestamp.date_naive(), date2.date_naive());
    assert_eq!(workouts[0].duration_minutes, Some(30));
    assert_eq!(workouts[0].distance, Some(5.0)); // Check stored distance (km)

    assert_eq!(workouts[1].timestamp.date_naive(), date1.date_naive());
    assert_eq!(workouts[1].duration_minutes, Some(60));
    assert_eq!(workouts[1].distance, Some(10.5));

    Ok(())
}

#[test]
fn test_add_workout_imperial_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Imperial; // Set units to Imperial

    service.create_exercise("Cycling", ExerciseType::Cardio, None, None)?;
    let today = Utc::now();

    let miles_input = 10.0;
    let expected_km = miles_input * 1.60934;

    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Cycling",
        date: today,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(45),
        distance: Some(miles_input), // Input in miles
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;

    // List workout and check stored distance (should be km)
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Cycling"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert!(workouts[0].distance.is_some());
    assert!((workouts[0].distance.unwrap() - expected_km).abs() < 0.001);

    Ok(())
}

#[test]
fn test_edit_workout_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Metric;
    service.create_exercise("Walking", ExerciseType::Cardio, None, None)?;
    let today = Utc::now();

    let (workout_id, _) = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Walking",
        date: today,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(60),
        distance: Some(5.0), // Input km
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;

    // Edit distance (Metric input)
    let new_distance_km = 7.5;
    let mut params = EditWorkoutParams::default();
    params.id = workout_id;
    params.new_distance_arg = Some(new_distance_km);
    service.edit_workout(params)?;

    // Verify (stored value is km)
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Walking"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, workout_id);
    assert_eq!(workouts[0].distance, Some(new_distance_km));

    // Edit distance with Imperial units input
    service.config.units = Units::Imperial;
    let imperial_input = 2.0; // 2 miles
    let expected_km_edit = imperial_input * 1.60934;
    let mut params = EditWorkoutParams::default();
    params.id = workout_id;
    params.new_distance_arg = Some(imperial_input);
    service.edit_workout(params)?;

    // Verify stored km
    let workouts_imperial_edit = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Walking"),
        ..Default::default()
    })?;
    assert_eq!(workouts_imperial_edit.len(), 1);
    assert!(workouts_imperial_edit[0].distance.is_some());
    assert!((workouts_imperial_edit[0].distance.unwrap() - expected_km_edit).abs() < 0.001);

    Ok(())
}

#[test]
fn test_bodyweight_workouts() -> Result<()> {
    let mut service = create_test_service()?;
    // Bodyweight is set to Some(70.0) in create_test_service

    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, None, Some("back"))?;
    let naive_date = NaiveDate::from_ymd_opt(2023, 6, 2).unwrap();
    let naive_datetime = naive_date.and_hms_opt(0, 0, 0).unwrap();
    let day1: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

    // Add workout with additional weight
    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Pull-ups",
        date: day1,
        sets: Some(3),
        reps: Some(10),
        weight: Some(5.0), // Additional weight
        duration: None,
        distance: None,
        notes: None,
        implicit_type: Some(ExerciseType::BodyWeight),
        implicit_muscles: None,
        bodyweight_to_use: Some(70.0), // Explicitly pass BW for test clarity
    })?;

    // Check that weight was calculated correctly
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Pull-ups"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 1);
    println!("here");
    assert_eq!(workouts[0].calculate_effective_weight(), Some(75.0)); // 70 + 5

    Ok(())
}

#[test]
fn test_edit_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    service.create_exercise("Bench Press", ExerciseType::Resistance, None, Some("chest"))?;

    // Edit the exercise
    service.edit_exercise(
        "Bench Press",
        Some("Barbell Bench Press"),
        Some(ExerciseType::Resistance),
        None,                                  // Keep type
        Some(Some("chest,triceps,shoulders")), // Update muscles
    )?;

    // Verify changes
    let exercise = service
        .get_exercise_by_identifier_service("Barbell Bench Press")?
        .unwrap();
    assert_eq!(exercise.name, "Barbell Bench Press");
    assert_eq!(
        exercise.muscles,
        Some("chest,triceps,shoulders".to_string())
    );

    // Try editing non-existent exercise
    let edit_result = service.edit_exercise("NonExistent", Some("WontWork"), None, None, None);
    assert!(edit_result.is_err());
    assert!(matches!(
        edit_result.unwrap_err().downcast_ref::<DbError>(),
        Some(DbError::ExerciseNotFound(_))
    ));

    Ok(())
}

#[test]
fn test_delete_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    service.create_exercise("Bench Press", ExerciseType::Resistance, None, Some("chest"))?;

    // Delete it
    let result = service.delete_exercise(&["Bench Press".to_string()])?;
    assert_eq!(result, 1);

    // Verify it's gone
    let exercise = service.get_exercise_by_identifier_service("Bench Press")?;
    assert!(exercise.is_none());

    // Try deleting non-existent exercise
    let delete_result = service.delete_exercise(&["NonExistent".to_string()]);
    assert!(delete_result.is_err());
    assert!(matches!(
        delete_result.unwrap_err().downcast_ref::<DbError>(),
        Some(DbError::ExerciseNotFound(_))
    ));

    Ok(())
}

#[test]
fn test_workout_filters() -> Result<()> {
    let mut service = create_test_service()?;

    service.create_exercise("Running", ExerciseType::Cardio, None, Some("legs"))?;
    service.create_exercise("Curl", ExerciseType::Resistance, None, Some("Biceps"))?;
    let naive_date = NaiveDate::from_ymd_opt(2023, 6, 3).unwrap();
    let naive_datetime = naive_date.and_hms_opt(0, 0, 0).unwrap();
    let date1: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

    // Add workouts on same date (ensure different timestamps implicitly)
    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Running",
        date: date1,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(30),
        distance: Some(5.0),
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;
    thread::sleep(StdDuration::from_millis(10)); // Ensure different timestamp
    service.add_workout(AddWorkoutParams {
        exercise_identifier: "Curl",
        date: date1,
        sets: Some(3),
        reps: Some(12),
        weight: Some(15.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;

    // Filter by type
    let resistance_workouts = service.list_workouts(&WorkoutFilters {
        exercise_type: Some(ExerciseType::Resistance),
        ..Default::default()
    })?;
    assert_eq!(resistance_workouts.len(), 1);
    assert_eq!(resistance_workouts[0].exercise_name, "Curl");

    let cardio_workouts = service.list_workouts(&WorkoutFilters {
        exercise_type: Some(ExerciseType::Cardio),
        ..Default::default()
    })?;
    assert_eq!(cardio_workouts.len(), 1);
    assert_eq!(cardio_workouts[0].exercise_name, "Running");

    // Filter by date
    let date1_workouts = service.list_workouts(&WorkoutFilters {
        date: Some(date1.date_naive()),
        ..Default::default()
    })?;
    assert_eq!(date1_workouts.len(), 2); // Both workouts on this date
                                         // Check order (ASC for date filter)
    assert_eq!(date1_workouts[0].exercise_name, "Running"); // Earlier timestamp
    assert_eq!(date1_workouts[1].exercise_name, "Curl"); // Later timestamp

    // Filter by muscle
    let leg_workouts = service.list_workouts(&WorkoutFilters {
        muscle: Some("legs"),
        ..Default::default()
    })?;
    assert_eq!(leg_workouts.len(), 1);
    assert_eq!(leg_workouts[0].exercise_name, "Running");

    let biceps_workouts = service.list_workouts(&WorkoutFilters {
        muscle: Some("Biceps"), // Test case-insensitivity of filter if DB is set up for it (depends on LIKE collation)
        ..Default::default()
    })?;
    assert_eq!(biceps_workouts.len(), 1);
    assert_eq!(biceps_workouts[0].exercise_name, "Curl");

    Ok(())
}

#[test]
fn test_nth_last_day_workouts() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Squats", ExerciseType::Resistance, None, Some("legs"))?;
    let date1 = NaiveDate::from_ymd_opt(2023, 6, 2).unwrap();
    let naive_datetime = date1.and_hms_opt(0, 0, 0).unwrap();
    let date1: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let date2 = NaiveDate::from_ymd_opt(2023, 6, 7).unwrap();
    let naive_datetime = date2.and_hms_opt(0, 0, 0).unwrap();
    let date2: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let date3 = NaiveDate::from_ymd_opt(2023, 6, 9).unwrap(); // Most recent
    let naive_datetime = date3.and_hms_opt(0, 0, 0).unwrap();
    let date3: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

    // Helper
    let mut add_squat = |date: DateTime<Utc>, sets: i64, weight: f64| -> Result<()> {
        service.add_workout(AddWorkoutParams {
            exercise_identifier: "Squats",
            date,
            sets: Some(sets),
            reps: Some(5),
            weight: Some(weight),
            ..Default::default()
        })?;
        Ok(())
    };

    // Add workouts
    add_squat(date1, 3, 100.0)?;
    add_squat(date2, 5, 120.0)?;
    add_squat(date3, 4, 125.0)?;
    add_squat(date3, 1, 130.0)?; // Add another on the most recent day

    // Get workouts for the most recent day (n=1)
    let recent_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 1)?;
    assert_eq!(recent_workouts.len(), 2); // Both workouts from date3
    assert_eq!(recent_workouts[0].timestamp, date3);
    assert_eq!(recent_workouts[0].sets, Some(4)); // First workout on date3
    assert_eq!(recent_workouts[1].timestamp, date3);
    assert_eq!(recent_workouts[1].sets, Some(1)); // Second workout on date3

    // Get workouts for the second most recent day (n=2)
    let previous_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 2)?;
    assert_eq!(previous_workouts.len(), 1);
    assert_eq!(previous_workouts[0].sets, Some(5));
    assert_eq!(previous_workouts[0].timestamp, date2);

    // Get workouts for the third most recent day (n=3)
    let oldest_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 3)?;
    assert_eq!(oldest_workouts.len(), 1);
    assert_eq!(oldest_workouts[0].sets, Some(3));
    assert_eq!(oldest_workouts[0].timestamp, date1);

    // Try getting n=4 (should be empty)
    let no_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 4)?;
    assert!(no_workouts.is_empty());

    // Try getting n=0 (should error)
    let zero_n_result = service.list_workouts_for_exercise_on_nth_last_day("Squats", 0);
    assert!(zero_n_result.is_err());
    // assert!(matches!(
    //     zero_n_result.unwrap_err().downcast_ref::<DbError>(),
    //     Some(DbError::InvalidParameterCount(0, 1))
    // ));

    Ok(())
}

#[test]
fn test_config_operations() -> Result<()> {
    let mut service = create_test_service()?;

    // Test setting bodyweight
    service.set_bodyweight(75.5)?;
    assert_eq!(service.config.bodyweight, Some(75.5));

    // Test getting required bodyweight
    let bw = service.get_required_bodyweight()?;
    assert_eq!(bw, 75.5);

    // Test disabling prompt
    service.disable_bodyweight_prompt()?;
    assert!(!service.config.prompt_for_bodyweight);

    // Test setting streak interval
    assert_eq!(service.config.streak_interval_days, 1); // Default
    service.set_streak_interval(3)?;
    assert_eq!(service.config.streak_interval_days, 3);
    let interval_result = service.set_streak_interval(0); // Test invalid interval
    assert!(interval_result.is_err());
    assert!(matches!(
        interval_result,
        Err(ConfigError::InvalidStreakInterval(0))
    ));

    Ok(())
}

#[test]
fn test_exercise_not_found() -> Result<()> {
    let mut service = create_test_service()?;

    // Try to get non-existent exercise
    let result = service.get_exercise_by_identifier_service("Non-existent");
    assert!(result.is_ok() && result?.is_none()); // Should be Ok(None)

    // Try to edit non-existent exercise
    let result = service.edit_exercise("Non-existent", None, None, None, None);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().downcast_ref::<DbError>(),
        Some(DbError::ExerciseNotFound(_))
    ));

    // Try to delete non-existent exercise
    let result = service.delete_exercise(&["Non-existent".to_string()]);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().downcast_ref::<DbError>(),
        Some(DbError::ExerciseNotFound(_))
    ));

    // Try to add workout for non-existent exercise without implicit details
    let result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Non-existent",
        date: Utc::now(),
        sets: Some(1),
        reps: Some(1),
        weight: None,
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    });
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found. Define it first"));

    Ok(())
}

#[test]
fn test_workout_not_found() -> Result<()> {
    let service = create_test_service()?; // Renamed from create_mutable_conn_to_test_db

    // Try to edit non-existent workout
    let result = service.edit_workout(EditWorkoutParams {
        id: 999,
        ..Default::default()
    });
    assert!(result.is_err());
    // assert!(matches!(
    //     result.unwrap_err().downcast_ref::<DbError>(),
    //     Some(DbError::WorkoutNotFound(999))
    // ));

    // Try to delete non-existent workout
    let result = service.delete_workouts(&[999]);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().downcast_ref::<DbError>(),
        Some(DbError::WorkoutNotFound(999))
    ));

    Ok(())
}

#[test]
fn test_bodyweight_validation() -> Result<()> {
    let mut service = create_test_service()?;

    // Test invalid bodyweight in config set
    let result = service.set_bodyweight(0.0);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ConfigError::InvalidBodyweightInput(_))
    ));

    let result = service.set_bodyweight(-10.0);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ConfigError::InvalidBodyweightInput(_))
    ));

    // Test invalid bodyweight in add_bodyweight_entry
    let result_add = service.add_bodyweight_entry(Utc::now(), 0.0);
    assert!(result_add.is_err());
    assert!(matches!(
        result_add.unwrap_err().downcast_ref::<ConfigError>(),
        Some(ConfigError::InvalidBodyweightInput(_))
    ));

    Ok(())
}

#[test]
fn test_set_units() -> Result<()> {
    let mut service = create_test_service()?;
    assert_eq!(service.config.units, Units::Metric);

    service.set_units(Units::Imperial)?;
    assert_eq!(service.config.units, Units::Imperial);

    service.set_units(Units::Metric)?;
    assert_eq!(service.config.units, Units::Metric);

    Ok(())
}

#[test]
fn test_workout_volume() -> Result<()> {
    let mut service = create_test_service()?;
    let day1 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
    let naive_datetime = day1.and_hms_opt(0, 0, 0).unwrap();
    let day1: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let day2 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
    let naive_datetime = day2.and_hms_opt(0, 0, 0).unwrap();
    let day2: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

    service.create_exercise("Bench Press", ExerciseType::Resistance, None, Some("chest"))?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, None, Some("back"))?;
    service.create_exercise("Running", ExerciseType::Cardio, None, Some("legs"))?;
    service.create_exercise("Squats", ExerciseType::Resistance, None, Some("legs"))?;

    // Helper
    let mut add_workout = |params: AddWorkoutParams| service.add_workout(params);

    // Day 1
    add_workout(AddWorkoutParams {
        exercise_identifier: "Bench Press",
        date: day1,
        sets: Some(3),
        reps: Some(10),
        weight: Some(100.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?; // Vol=3000
    add_workout(AddWorkoutParams {
        exercise_identifier: "Bench Press",
        date: day1,
        sets: Some(1),
        reps: Some(8),
        weight: Some(105.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?; // Vol=840
    add_workout(AddWorkoutParams {
        exercise_identifier: "Pull-ups",
        date: day1,
        sets: Some(4),
        reps: Some(6),
        weight: Some(10.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: Some(70.0),
    })?; // Vol=1920
    add_workout(AddWorkoutParams {
        exercise_identifier: "Running",
        date: day1,
        sets: None,
        reps: None,
        weight: None,
        duration: Some(30),
        distance: Some(5.0),
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?; // Vol=0

    // Day 2
    add_workout(AddWorkoutParams {
        exercise_identifier: "Squats",
        date: day2,
        sets: Some(5),
        reps: Some(5),
        weight: Some(120.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?; // Vol=3000
    add_workout(AddWorkoutParams {
        exercise_identifier: "Bench Press",
        date: day2,
        sets: Some(4),
        reps: Some(6),
        weight: Some(100.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?; // Vol=2400

    // --- Test Volume Calculation ---

    // Total volume (no filters)
    let volume_all = service.calculate_daily_volume(&VolumeFilters::default())?;
    // Expected: (day2, BP, 2400), (day2, Squats, 3000), (day1, BP, 3840), (day1, Pull-ups, 1920), (day1, Running, 0) - Order: Date DESC, Name ASC
    assert_eq!(volume_all.len(), 5);
    // Day 2
    assert_eq!(
        volume_all[0],
        (day2.date_naive(), "Bench Press".to_string(), 2400.0)
    );
    assert_eq!(
        volume_all[1],
        (day2.date_naive(), "Squats".to_string(), 3000.0)
    );
    // Day 1
    assert_eq!(
        volume_all[2],
        (day1.date_naive(), "Bench Press".to_string(), 3840.0)
    );
    assert_eq!(
        volume_all[3],
        (day1.date_naive(), "Pull-ups".to_string(), 1920.0)
    );
    assert_eq!(
        volume_all[4],
        (day1.date_naive(), "Running".to_string(), 0.0)
    );

    // Volume for Day 1 only
    let volume_day1 = service.calculate_daily_volume(&VolumeFilters {
        start_date: Some(day1.date_naive()),
        end_date: Some(day1.date_naive()),
        ..Default::default()
    })?;
    assert_eq!(volume_day1.len(), 3); // BP, Pull-ups, Running
    assert_eq!(volume_day1[0].1, "Bench Press");
    assert_eq!(volume_day1[1].1, "Pull-ups");
    assert_eq!(volume_day1[2].1, "Running");

    // Volume for "Bench Press" only
    let volume_bp = service.calculate_daily_volume(&VolumeFilters {
        exercise_name: Some("Bench Press"),
        ..Default::default()
    })?;
    assert_eq!(volume_bp.len(), 2); // BP on day 2 and day 1
    assert_eq!(
        volume_bp[0],
        (day2.date_naive(), "Bench Press".to_string(), 2400.0)
    ); // Day 2 first
    assert_eq!(
        volume_bp[1],
        (day1.date_naive(), "Bench Press".to_string(), 3840.0)
    );

    // Volume for Cardio (should be 0)
    let volume_cardio = service.calculate_daily_volume(&VolumeFilters {
        exercise_type: Some(ExerciseType::Cardio),
        ..Default::default()
    })?;
    assert_eq!(volume_cardio.len(), 1);
    assert_eq!(
        volume_cardio[0],
        (day1.date_naive(), "Running".to_string(), 0.0)
    );

    Ok(())
}

#[test]
fn test_exercise_stats() -> Result<()> {
    let mut service = create_test_service()?;
    let day1 = NaiveDate::from_ymd_opt(2023, 10, 20).unwrap(); // Fri
    let naive_datetime = day1.and_hms_opt(0, 0, 0).unwrap();
    let day1: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let day2 = NaiveDate::from_ymd_opt(2023, 10, 22).unwrap(); // Sun (Gap 1 day)
    let naive_datetime = day2.and_hms_opt(0, 0, 0).unwrap();
    let day2: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let day3 = NaiveDate::from_ymd_opt(2023, 10, 23).unwrap(); // Mon (Gap 0 days)
    let naive_datetime = day3.and_hms_opt(0, 0, 0).unwrap();
    let day3: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let day4 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(); // Fri (Gap 3 days) - Longest Gap 3
    let naive_datetime = day4.and_hms_opt(0, 0, 0).unwrap();
    let day4: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let day5 = NaiveDate::from_ymd_opt(2023, 10, 28).unwrap(); // Sat (Gap 0 days)
    let naive_datetime = day5.and_hms_opt(0, 0, 0).unwrap();
    let day5: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

    service.create_exercise(
        "Test Stats",
        ExerciseType::Resistance,
        Some((Some(true), Some(true), Some(true), Some(true))),
        None,
    )?;

    // Helper
    let mut add_test_workout = |date: DateTime<Utc>,
                                reps: i64,
                                weight: f64,
                                dur: Option<i64>,
                                dist: Option<f64>|
     -> Result<()> {
        service.add_workout(AddWorkoutParams {
            exercise_identifier: "Test Stats",
            date,
            sets: Some(3),
            reps: Some(reps),
            weight: Some(weight),
            duration: dur,
            distance: dist,
            notes: None,
            implicit_type: None,
            implicit_muscles: None,
            bodyweight_to_use: None,
        })?;
        thread::sleep(StdDuration::from_millis(5)); // Ensure unique timestamp
        Ok(())
    };

    // Add workouts
    add_test_workout(day1, 10, 50.0, None, None)?; // PB: W=50, R=10
    add_test_workout(day2, 8, 55.0, Some(10), None)?; // PB: W=55, D=10
    add_test_workout(day3, 6, 50.0, Some(12), None)?; // PB: D=12
    add_test_workout(day4, 12, 45.0, None, Some(5.0))?; // PB: R=12, Dist=5.0
    add_test_workout(day5, 10, 55.0, Some(10), Some(5.5))?; // PB: Dist=5.5

    // --- Test with daily streak interval (default) ---
    let stats_daily = service.get_exercise_stats("Test Stats")?;

    assert_eq!(stats_daily.canonical_name, "Test Stats");
    assert_eq!(stats_daily.total_workouts, 5);
    assert_eq!(stats_daily.first_workout_date, Some(day1.date_naive()));
    assert_eq!(stats_daily.last_workout_date, Some(day5.date_naive()));

    // Avg/week: 5 workouts / (8 days / 7 days/week) = 35/8 = 4.375
    assert!((stats_daily.avg_workouts_per_week.unwrap() - 4.375).abs() < 0.01);

    // Longest gap days (days *between*): (day2-day1)-1 = 1, (day3-day2)-1 = 0, (day4-day3)-1 = 3, (day5-day4)-1 = 0 -> Max = 3
    assert_eq!(stats_daily.longest_gap_days, Some(3));

    // Streaks (daily interval = 1 day):
    // day1(F)->day2(Su) (gap 1 day <= 1) YES [streak=d1,d2 = 2] last=d2
    // day2(Su)->day3(M) (gap 0 days <= 1) YES [streak=d1,d2,d3 = 3] last=d3
    // day3(M)->day4(F) (gap 3 days > 1) NO [streak=d4 = 1] last=d4
    // day4(F)->day5(S) (gap 0 days <= 1) YES [streak=d4,d5 = 2] last=d5
    // Longest = 3. Current = 0 (assuming test runs after day5)
    assert_eq!(stats_daily.current_streak, 0);
    assert_eq!(stats_daily.longest_streak, 2);
    assert_eq!(stats_daily.streak_interval_days, 1);

    // PBs
    assert_eq!(stats_daily.personal_bests.max_weight, Some(55.0));
    assert_eq!(stats_daily.personal_bests.max_reps, Some(12));
    assert_eq!(stats_daily.personal_bests.max_duration_minutes, Some(12));
    assert_eq!(stats_daily.personal_bests.max_distance_km, Some(5.5));

    // --- Test with 2-day streak interval ---
    service.set_streak_interval(2)?;
    let stats_2day = service.get_exercise_stats("Test Stats")?;

    assert_eq!(stats_2day.streak_interval_days, 2);
    // Streaks (2-day interval):
    // day1(F)->day2(Su) (gap 1 day <= 2) YES [streak=d1,d2 = 2] last=d2
    // day2(Su)->day3(M) (gap 0 days <= 2) YES [streak=d1,d2,d3 = 3] last=d3
    // day3(M)->day4(F) (gap 3 days > 2) NO [streak=d4 = 1] last=d4
    // day4(F)->day5(S) (gap 0 days <= 2) YES [streak=d4,d5 = 2] last=d5
    // Longest = 3. Current = 0
    assert_eq!(stats_2day.current_streak, 0);
    assert_eq!(stats_2day.longest_streak, 3);

    // --- Test Edge Cases ---
    service.create_exercise("No Workouts", ExerciseType::Cardio, None, None)?;
    let no_workout_result = service.get_exercise_stats("No Workouts");
    assert!(no_workout_result.is_err());
    assert!(matches!(
        no_workout_result.unwrap_err().downcast_ref::<DbError>(),
        Some(DbError::NoWorkoutDataFound(_))
    ));

    // Test one workout
    service.create_exercise("One Workout", ExerciseType::Resistance, None, None)?;
    let day_single = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();
    let naive_datetime = day_single.and_hms_opt(0, 0, 0).unwrap();
    let day_single: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    service.add_workout(AddWorkoutParams {
        exercise_identifier: "One Workout",
        date: day_single,
        sets: Some(1),
        reps: Some(5),
        weight: Some(10.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None,
    })?;
    let one_stats = service.get_exercise_stats("One Workout")?;

    assert_eq!(one_stats.total_workouts, 1);
    assert!(one_stats.avg_workouts_per_week.is_none());
    assert!(one_stats.longest_gap_days.is_none());
    assert_eq!(one_stats.current_streak, 0);
    assert_eq!(one_stats.longest_streak, 1);
    assert_eq!(one_stats.personal_bests.max_weight, Some(10.0));

    Ok(())
}

#[test]
fn test_get_latest_bodyweight() -> Result<()> {
    let service = create_test_service()?;

    assert!(service.get_latest_bodyweight()?.is_none());

    service.add_bodyweight_entry(Utc::now() - Duration::days(2), 70.0)?;
    thread::sleep(StdDuration::from_millis(10));
    service.add_bodyweight_entry(Utc::now() - Duration::days(1), 71.0)?; // Latest

    let latest = service.get_latest_bodyweight()?;
    assert_eq!(latest, Some(71.0));

    Ok(())
}

#[test]
fn test_delete_body_weight() -> Result<()> {
    let mut service = create_test_service()?;
    let id1 = service.add_bodyweight_entry(Utc::now() - Duration::days(2), 50.0)?;
    let result = service.delete_bodyweight(id1)?;
    assert_eq!(result, 1); // 1 row affected

    // Try deleting again
    let result_err = service.delete_bodyweight(id1);
    assert!(result_err.is_err());
    assert!(matches!(
        result_err,
        Err(DbError::BodyWeightEntryNotFound(_))
    ));

    // Try deleting non-existent
    let result_err2 = service.delete_bodyweight(999);
    assert!(result_err2.is_err());
    assert!(matches!(
        result_err2,
        Err(DbError::BodyWeightEntryNotFound(999))
    ));

    Ok(())
}

#[test]
fn test_bodyweight_workout_needs_log() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, None, Some("back"))?;

    // Test config BW initially None
    service.config.bodyweight = None;

    // Try adding BW workout without a logged BW or config BW
    let result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Pull-ups",
        date: Utc::now(),
        sets: Some(3),
        reps: Some(5),
        weight: Some(10.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: None, // Simulate main not finding a logged BW
    });

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Bodyweight log required"));

    // Log bodyweight
    service.add_bodyweight_entry(Utc::now(), 75.0)?;
    let logged_bw = service.get_latest_bodyweight()?.unwrap(); // Should be 75.0

    // Try adding again, simulating main providing the logged BW
    let add_result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Pull-ups",
        date: Utc::now(),
        sets: Some(3),
        reps: Some(5),
        weight: Some(10.0),
        duration: None,
        distance: None,
        notes: None,
        implicit_type: None,
        implicit_muscles: None,
        bodyweight_to_use: Some(logged_bw), // Pass the fetched BW
    });

    assert!(add_result.is_ok());
    let (id, _) = add_result?;

    // Verify workout weight calculation
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Pull-ups"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, id);
    assert_eq!(workouts[0].calculate_effective_weight(), Some(85.0)); // 75.0 (logged) + 10.0 (additional)

    Ok(())
}

#[test]
fn test_add_list_bodyweight() -> Result<()> {
    let service = create_test_service()?;
    let date1 = Utc::now() - Duration::days(2);
    let date2 = Utc::now() - Duration::days(1);
    let date3 = Utc::now();

    // Add entries with slight delay
    let id1 = service.add_bodyweight_entry(date1, 70.5)?;
    thread::sleep(StdDuration::from_millis(10));
    let id2 = service.add_bodyweight_entry(date2, 71.0)?;
    thread::sleep(StdDuration::from_millis(10));
    let id3 = service.add_bodyweight_entry(date3, 70.8)?;

    // List entries (limit 10)
    let entries = service.list_bodyweights(10)?;
    assert_eq!(entries.len(), 3);

    // Check order (descending by timestamp) and IDs
    assert_eq!(entries[0].0, id3); // Most recent ID
    assert_eq!(entries[0].2, 70.8);
    assert!((entries[0].1 - date3).num_milliseconds().abs() < 100);

    assert_eq!(entries[1].0, id2);
    assert_eq!(entries[1].2, 71.0);
    assert!((entries[1].1 - date2).num_milliseconds().abs() < 100);

    assert_eq!(entries[2].0, id1);
    assert_eq!(entries[2].2, 70.5);
    assert!((entries[2].1 - date1).num_milliseconds().abs() < 100);

    // Test limit
    let limited_entries = service.list_bodyweights(1)?;
    assert_eq!(limited_entries.len(), 1);
    assert_eq!(limited_entries[0].0, id3); // Should be the latest one
    assert_eq!(limited_entries[0].2, 70.8);

    Ok(())
}

#[test]
fn test_target_bodyweight_config() -> Result<()> {
    let mut service = create_test_service()?;

    assert!(service.get_target_bodyweight().is_none());

    // Set target
    service.set_target_bodyweight(Some(78.5))?;
    assert_eq!(service.config.target_bodyweight, Some(78.5));
    assert_eq!(service.get_target_bodyweight(), Some(78.5));

    // Clear target
    service.set_target_bodyweight(None)?;
    assert!(service.config.target_bodyweight.is_none());
    assert!(service.get_target_bodyweight().is_none());

    // Test invalid input
    let result_neg = service.set_target_bodyweight(Some(-10.0));
    assert!(result_neg.is_err());
    assert!(matches!(
        result_neg,
        Err(ConfigError::InvalidBodyweightInput(_))
    ));

    Ok(())
}

#[test]
fn get_all_dates_exercised() -> Result<()> {
    let mut service = create_test_service()?;
    let today = Utc::now().date_naive();
    let naive_datetime = today.and_hms_opt(0, 0, 0).unwrap();
    let today: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);
    let yesterday = today - Duration::days(1);
    service.create_exercise("Bench Press", ExerciseType::Resistance, None, Some("chest"))?;

    // Helper
    let mut add_bench = |date: DateTime<Utc>| -> Result<()> {
        service.add_workout(AddWorkoutParams {
            exercise_identifier: "Bench Press",
            date,
            sets: None,
            reps: None,
            weight: None,
            duration: None,
            distance: None,
            notes: None,
            implicit_type: None,
            implicit_muscles: None,
            bodyweight_to_use: None,
        })?;
        thread::sleep(StdDuration::from_millis(5)); // Ensure unique timestamp if needed
        Ok(())
    };

    add_bench(today)?;
    add_bench(today)?; // Add again on same day
    add_bench(yesterday)?;

    let dates = service.get_all_dates_with_exercise()?;
    assert_eq!(dates.len(), 2); // Should only contain unique dates
    assert_eq!(dates[0], yesterday.date_naive()); // Should be sorted ASC
    assert_eq!(dates[1], today.date_naive());

    Ok(())
}

#[test]
fn test_graph_data_fetching() -> Result<()> {
    // Assuming Result is anyhow::Result or similar
    let mut service = create_test_service()?; // Assuming this function is available

    // Define NaiveDate for assertions
    let date_2023_10_26 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
    let date_2023_10_27 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
    let date_2023_10_28 = NaiveDate::from_ymd_opt(2023, 10, 28).unwrap();

    // Define DateTime<Utc> for workout entries
    // Using specific times to ensure AddWorkoutParams.date has a time component.
    // The NaiveDate part is what matters for aggregation.
    let dt_2023_10_26: DateTime<Utc> =
        DateTime::from_naive_utc_and_offset(date_2023_10_26.and_hms_opt(9, 0, 0).unwrap(), Utc);
    let dt_2023_10_27: DateTime<Utc> =
        DateTime::from_naive_utc_and_offset(date_2023_10_27.and_hms_opt(10, 0, 0).unwrap(), Utc);
    let dt_2023_10_28: DateTime<Utc> =
        DateTime::from_naive_utc_and_offset(date_2023_10_28.and_hms_opt(11, 0, 0).unwrap(), Utc);

    service.create_exercise("Bench Press", ExerciseType::Resistance, None, Some("chest"))?;
    service.create_exercise("Running", ExerciseType::Cardio, None, Some("legs"))?;

    // Helper
    let mut add_workout = |params: AddWorkoutParams| -> Result<()> {
        service.add_workout(params)?;
        thread::sleep(StdDuration::from_millis(5)); // Ensure unique timestamp if processing relies on it
        Ok(())
    };

    // Bench Press data
    add_workout(AddWorkoutParams {
        exercise_identifier: "Bench Press",
        date: dt_2023_10_26, // Use DateTime<Utc>
        sets: Some(3),
        reps: Some(10),
        weight: Some(100.0),
        ..Default::default()
    })?;
    add_workout(AddWorkoutParams {
        exercise_identifier: "Bench Press",
        date: dt_2023_10_27, // Use DateTime<Utc>
        sets: Some(4),
        reps: Some(8),
        weight: Some(105.0),
        ..Default::default()
    })?;
    add_workout(AddWorkoutParams {
        exercise_identifier: "Bench Press",
        date: dt_2023_10_27, // Use DateTime<Utc>
        sets: Some(1),
        reps: Some(6),
        weight: Some(110.0),
        ..Default::default()
    })?;
    add_workout(AddWorkoutParams {
        exercise_identifier: "Bench Press",
        date: dt_2023_10_28, // Use DateTime<Utc>
        sets: Some(2),
        reps: Some(5),
        weight: Some(110.0),
        ..Default::default()
    })?;

    // Running data
    add_workout(AddWorkoutParams {
        exercise_identifier: "Running",
        date: dt_2023_10_27, // Use DateTime<Utc>
        duration: Some(30),
        distance: Some(5.0),
        ..Default::default()
    })?;
    add_workout(AddWorkoutParams {
        exercise_identifier: "Running",
        date: dt_2023_10_27, // Use DateTime<Utc>
        duration: Some(10),
        distance: Some(2.0),
        ..Default::default()
    })?;
    add_workout(AddWorkoutParams {
        exercise_identifier: "Running",
        date: dt_2023_10_28, // Use DateTime<Utc>
        duration: Some(35),
        distance: Some(5.5),
        ..Default::default()
    })?;

    // Test E1RM
    let e1rm_data =
        service.get_data_for_graph("Bench Press", GraphType::Estimated1RM, None, None)?; // Added None, None
    assert_eq!(
        e1rm_data,
        vec![
            (date_2023_10_26, 133.33333333333331), // Use NaiveDate
            (date_2023_10_27, 133.0),              // Use NaiveDate
            (date_2023_10_28, 128.33333333333334)  // Use NaiveDate
        ]
    );

    // Test Max Weight
    let weight_data =
        service.get_data_for_graph("Bench Press", GraphType::MaxWeight, None, None)?; // Added None, None
    assert_eq!(
        weight_data,
        vec![
            (date_2023_10_26, 100.0), // Use NaiveDate
            (date_2023_10_27, 110.0), // Use NaiveDate
            (date_2023_10_28, 110.0)  // Use NaiveDate
        ]
    );

    // Test Max Reps
    let reps_data = service.get_data_for_graph("Bench Press", GraphType::MaxReps, None, None)?; // Added None, None
    assert_eq!(
        reps_data,
        vec![
            (date_2023_10_26, 10.0), // Use NaiveDate
            (date_2023_10_27, 8.0),  // Use NaiveDate
            (date_2023_10_28, 5.0)   // Use NaiveDate
        ]
    );

    // Test Workout Volume
    let volume_data =
        service.get_data_for_graph("Bench Press", GraphType::WorkoutVolume, None, None)?; // Added None, None
    assert_eq!(
        volume_data,
        vec![
            (date_2023_10_26, 3000.0), // Use NaiveDate
            (date_2023_10_27, 4020.0), // Use NaiveDate
            (date_2023_10_28, 1100.0)  // Use NaiveDate
        ]
    );

    // Test Workout Reps (Total reps)
    let workout_reps_data =
        service.get_data_for_graph("Bench Press", GraphType::WorkoutReps, None, None)?; // Added None, None
    assert_eq!(
        workout_reps_data,
        vec![
            (date_2023_10_26, 30.0), // Use NaiveDate
            (date_2023_10_27, 38.0), // Use NaiveDate
            (date_2023_10_28, 10.0)  // Use NaiveDate
        ]
    );

    // Test Workout Duration (Running - Summed)
    let duration_data =
        service.get_data_for_graph("Running", GraphType::WorkoutDuration, None, None)?; // Added None, None
    assert_eq!(
        duration_data,
        vec![
            (date_2023_10_27, 40.0), // Running data starts on 27th
            (date_2023_10_28, 35.0)
        ]
    );

    // Test Workout Distance (Running - Metric - Summed)
    let distance_data_metric =
        service.get_data_for_graph("Running", GraphType::WorkoutDistance, None, None)?; // Added None, None
    assert_eq!(
        distance_data_metric,
        vec![
            (date_2023_10_27, 7.0), // Running data starts on 27th
            (date_2023_10_28, 5.5)
        ]
    );

    // Test Workout Distance (Running - Imperial - Summed)
    service.config.units = Units::Imperial; // Assuming Units enum is available
    let distance_data_imperial =
        service.get_data_for_graph("Running", GraphType::WorkoutDistance, None, None)?; // Added None, None
    assert_eq!(distance_data_imperial.len(), 2);
    assert_eq!(distance_data_imperial[0].0, date_2023_10_27); // Compare with NaiveDate
    assert!((distance_data_imperial[0].1 - (7.0 * 0.621_371)).abs() < 0.01); // miles
    assert_eq!(distance_data_imperial[1].0, date_2023_10_28); // Compare with NaiveDate
    assert!((distance_data_imperial[1].1 - (5.5 * 0.621_371)).abs() < 0.01); // miles

    // Test for exercise with no data
    service.create_exercise("Untouched", ExerciseType::Resistance, None, None)?;
    let no_data = service.get_data_for_graph("Untouched", GraphType::MaxWeight, None, None)?; // Added None, None
    assert!(no_data.is_empty());

    Ok(())
}

#[test]
fn test_list_all_muscles() -> Result<()> {
    let service = create_test_service()?;

    assert!(service.list_all_muscles()?.is_empty());

    // Add exercises
    service.create_exercise(
        "Bench Press",
        ExerciseType::Resistance,
        None,
        Some("Chest, Triceps"),
    )?;
    service.create_exercise(
        "Squat",
        ExerciseType::Resistance,
        None,
        Some("Legs, Glutes, Core"),
    )?;
    service.create_exercise(
        "Pull-ups",
        ExerciseType::BodyWeight,
        None,
        Some("back, Biceps "),
    )?;
    service.create_exercise(
        "Rows",
        ExerciseType::Resistance,
        None,
        Some("Back, Rear Delts"),
    )?;
    service.create_exercise("Running", ExerciseType::Cardio, None, Some("Legs"))?;
    service.create_exercise("Crunches", ExerciseType::BodyWeight, None, Some("core"))?;
    service.create_exercise("Empty Muscle", ExerciseType::Resistance, None, Some(""))?;
    service.create_exercise("Null Muscle", ExerciseType::Resistance, None, None)?;
    service.create_exercise("Just Comma", ExerciseType::Resistance, None, Some(","))?;
    service.create_exercise(
        "Leading Comma",
        ExerciseType::Resistance,
        None,
        Some(",shoulders"),
    )?;

    let muscles = service.list_all_muscles()?;
    let expected_muscles = vec![
        "back",
        "biceps",
        "chest",
        "core",
        "glutes",
        "legs",
        "rear delts",
        "shoulders",
        "triceps",
    ];
    assert_eq!(muscles, expected_muscles);

    Ok(())
}

#[test]
fn test_log_flag_restrictions() -> Result<()> {
    let mut service = create_test_service()?;

    // Create a "Plank" exercise definition: log duration only (weight/reps/distance = false)
    // The log_flags tuple is (log_weight, log_reps, log_duration, log_distance)
    service.create_exercise(
        "Plank",
        ExerciseType::BodyWeight, // Type won't strictly matter for log flags, but semantically correct
        Some((Some(false), Some(false), Some(true), Some(false))),
        Some("core"),
    )?;

    // Get required bodyweight for BodyWeight exercise (assuming it's available in config or logged)
    // We provide this to avoid triggering the "bodyweight needed" error in this test,
    // as we want to test the *log flag* errors.
    service.set_bodyweight(70.0)?; // Ensure config has BW set
    let bodyweight_val = service.get_required_bodyweight()?;

    // --- Attempt 1: Add workout with only allowed data (duration) ---
    let allowed_result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Plank",
        date: Utc::now(),
        duration: Some(60),                      // This is allowed
        bodyweight_to_use: Some(bodyweight_val), // Required for BW type
        ..Default::default()                     // No weight, reps, distance provided
    });
    assert!(
        allowed_result.is_ok(),
        "Adding workout with only allowed duration should succeed"
    );
    // Optionally verify the workout was added and has the correct duration and bodyweight
    let workouts = service.list_workouts(&WorkoutFilters {
        exercise_name: Some("Plank"),
        limit: Some(1),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].duration_minutes, Some(60));
    assert_eq!(workouts[0].bodyweight, Some(bodyweight_val));
    assert!(workouts[0].weight.is_none()); // Check restricted fields are None
    assert!(workouts[0].reps.is_none());
    assert!(workouts[0].distance.is_none());

    // --- Attempt 2: Add workout with disallowed data (distance) ---
    let disallowed_dist_result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Plank",
        date: Utc::now(),
        duration: Some(60),                      // Allowed
        distance: Some(1.0),                     // NOT allowed by definition
        bodyweight_to_use: Some(bodyweight_val), // Required for BW type
        ..Default::default()
    });
    assert!(
        disallowed_dist_result.is_err(),
        "Adding workout with disallowed distance should fail"
    );
    let err_msg = disallowed_dist_result.unwrap_err().to_string();
    assert!(err_msg.contains("Exercise 'Plank' is not configured to log the following: distance"));

    // --- Attempt 3: Add workout with multiple disallowed data fields ---
    let disallowed_multiple_result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Plank",
        date: Utc::now(),
        duration: Some(60),                      // Allowed
        weight: Some(5.0),                       // NOT allowed
        reps: Some(10),                          // NOT allowed
        distance: Some(0.5),                     // NOT allowed
        bodyweight_to_use: Some(bodyweight_val), // Required for BW type
        ..Default::default()
    });
    assert!(
        disallowed_multiple_result.is_err(),
        "Adding workout with multiple disallowed fields should fail"
    );
    let err_msg_multiple = disallowed_multiple_result.unwrap_err().to_string();
    // Check that the error message lists all violations
    assert!(
        err_msg_multiple.contains(
            "Exercise 'Plank' is not configured to log the following: weight, reps, distance"
        ) || err_msg_multiple.contains(
            "Exercise 'Plank' is not configured to log the following: reps, weight, distance"
        ) || err_msg_multiple.contains(
            "Exercise 'Plank' is not configured to log the following: distance, weight, reps"
        ) // Check for common order variations
          // Add other order variations if needed, or sort the expected string for assertion
    );

    // --- Attempt 4: Add workout with *only* disallowed data ---
    let only_disallowed_result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Plank",
        date: Utc::now(),
        weight: Some(1.0),                       // NOT allowed
        reps: Some(1),                           // NOT allowed
        distance: Some(0.1),                     // NOT allowed
        bodyweight_to_use: Some(bodyweight_val), // Required for BW type
        ..Default::default()                     // duration: None, which is the *only* allowed field
    });
    assert!(
        only_disallowed_result.is_err(),
        "Adding workout with only disallowed data should fail"
    );
    let err_msg_only_disallowed = only_disallowed_result.unwrap_err().to_string();
    assert!(
        err_msg_only_disallowed.contains(
            "Exercise 'Plank' is not configured to log the following: weight, reps, distance"
        ) || err_msg_only_disallowed.contains(
            "Exercise 'Plank' is not configured to log the following: reps, weight, distance"
        ) || err_msg_only_disallowed.contains(
            "Exercise 'Plank' is not configured to log the following: distance, weight, reps"
        ) // Check for common order variations
    );

    // --- Create a different exercise that logs weight/reps but not duration/distance ---
    service.create_exercise(
        "Bicep Curl",
        ExerciseType::Resistance,
        Some((Some(true), Some(true), Some(false), Some(false))), // (w, r, dur, dist)
        Some("biceps"),
    )?;

    // Attempt to add duration data for Bicep Curl
    let disallowed_curl_duration_result = service.add_workout(AddWorkoutParams {
        exercise_identifier: "Bicep Curl",
        date: Utc::now(),
        sets: Some(3),
        reps: Some(10),
        weight: Some(20.0),
        duration: Some(5), // NOT allowed for Bicep Curl
        ..Default::default()
    });
    assert!(
        disallowed_curl_duration_result.is_err(),
        "Adding duration to Bicep Curl should fail"
    );
    let err_msg_curl = disallowed_curl_duration_result.unwrap_err().to_string();
    assert!(err_msg_curl
        .contains("Exercise 'Bicep Curl' is not configured to log the following: duration"));

    Ok(())
}

#[test]
fn test_get_workout_dates_for_month() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Test Exercise", ExerciseType::Resistance, None, None)?;

    // Helper to add a workout on a specific date
    let mut add_workout_on_date_str = |date_str: &str| -> Result<()> {
        let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        let naive_datetime = naive_date.and_hms_opt(12, 0, 0).unwrap(); // Noon
        let timestamp: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

        service.add_workout(AddWorkoutParams {
            exercise_identifier: "Test Exercise",
            date: timestamp,
            sets: Some(1),
            reps: Some(1),
            weight: Some(10.0),
            ..Default::default()
        })?;
        // Add a small delay to ensure timestamps are unique if adding multiple on same conceptual day
        // but for this test, we are testing distinct dates.
        thread::sleep(StdDuration::from_millis(5));
        Ok(())
    };

    // --- Add workouts for specific months ---
    // October 2023
    add_workout_on_date_str("2023-10-01")?;
    add_workout_on_date_str("2023-10-15")?;
    add_workout_on_date_str("2023-10-15")?; // Add another on same day, should still be one entry for the date
    add_workout_on_date_str("2023-10-31")?;

    // November 2023
    add_workout_on_date_str("2023-11-05")?;
    add_workout_on_date_str("2023-11-20")?;

    // December 2023 (no workouts)

    // January 2024
    add_workout_on_date_str("2024-01-10")?;

    // --- Test cases ---

    // 1. Month with workouts (October 2023)
    let oct_dates = service.get_workout_dates_for_month(2023, 10)?;
    assert_eq!(
        oct_dates.len(),
        3,
        "October should have 3 unique workout dates"
    );
    assert_eq!(oct_dates, vec!["2023-10-01", "2023-10-15", "2023-10-31"]); // Expect sorted

    // 2. Month with workouts (November 2023)
    let nov_dates = service.get_workout_dates_for_month(2023, 11)?;
    assert_eq!(
        nov_dates.len(),
        2,
        "November should have 2 unique workout dates"
    );
    assert_eq!(nov_dates, vec!["2023-11-05", "2023-11-20"]);

    // 3. Month with no workouts (December 2023)
    let dec_dates = service.get_workout_dates_for_month(2023, 12)?;
    assert!(
        dec_dates.is_empty(),
        "December should have no workout dates"
    );

    // 4. Month in a different year (January 2024)
    let jan_2024_dates = service.get_workout_dates_for_month(2024, 1)?;
    assert_eq!(
        jan_2024_dates.len(),
        1,
        "January 2024 should have 1 workout date"
    );
    assert_eq!(jan_2024_dates, vec!["2024-01-10"]);

    // 5. Test invalid month (e.g., month 0 or 13)
    let invalid_month_low = service.get_workout_dates_for_month(2023, 0);
    assert!(invalid_month_low.is_err(), "Month 0 should be an error");
    assert!(invalid_month_low
        .unwrap_err()
        .to_string()
        .contains("Invalid month"));

    let invalid_month_high = service.get_workout_dates_for_month(2023, 13);
    assert!(invalid_month_high.is_err(), "Month 13 should be an error");
    assert!(invalid_month_high
        .unwrap_err()
        .to_string()
        .contains("Invalid month"));

    // 6. Month with no workouts in a year that has other workouts
    let feb_2024_dates = service.get_workout_dates_for_month(2024, 2)?;
    assert!(
        feb_2024_dates.is_empty(),
        "February 2024 should have no workout dates"
    );

    Ok(())
}
