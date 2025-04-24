use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use rusqlite::Connection;
use std::thread; // For adding delays in PB tests
use std::time::Duration as StdDuration; // For delays
use task_athlete_lib::{
    AppService, Config, ConfigError, DbError, ExerciseType, GraphType, Units, VolumeFilters,
    WorkoutFilters,
}; // For mutable connection helper

// Helper function to create a test service with in-memory database
fn create_test_service() -> Result<AppService> {
    // Create an in-memory database for testing
    // Need a mutable connection to pass to init_db
    let mut conn = rusqlite::Connection::open_in_memory()?;
    task_athlete_lib::db::init_db(&mut conn)?; // Pass mutable conn

    // Create a default config for testing
    let config = Config {
        bodyweight: Some(70.0), // Set a default bodyweight for tests
        units: Units::Metric,
        prompt_for_bodyweight: true,
        notify_pb_enabled: Some(true), // Explicitly enable for most tests
        streak_interval_days: 1,       // Default streak interval
        notify_pb_weight: true,
        notify_pb_reps: true,
        notify_pb_duration: true,
        notify_pb_distance: true,
        ..Default::default()
    };

    Ok(AppService {
        config,
        conn, // Store the initialized connection
        db_path: ":memory:".into(),
        config_path: "test_config.toml".into(),
    })
}

// Helper to get a separate mutable connection for tests requiring transactions (like edit_exercise)
// This is a bit of a hack because AppService owns its connection.
fn create_mutable_conn_to_test_db() -> Result<Connection> {
    let mut conn = rusqlite::Connection::open_in_memory()?;
    task_athlete_lib::db::init_db(&mut conn)?; // Ensure schema is initialized
    Ok(conn)
}

#[test]
fn test_create_exercise_unique_name() -> Result<()> {
    let service = create_test_service()?;
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Try creating with same name (case-insensitive)
    let result = service.create_exercise("bench press", ExerciseType::Cardio, None);
    assert!(result.is_err());
    // Check for the specific error type/message if desired
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Exercise name must be unique"));

    // Try creating with different name
    let result = service.create_exercise("Squat", ExerciseType::Resistance, Some("legs"));
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_exercise_aliases() -> Result<()> {
    let mut service = create_test_service()?;

    let ex_id = service.create_exercise(
        "Barbell Bench Press",
        ExerciseType::Resistance,
        Some("chest"),
    )?;
    service.create_exercise("Squat", ExerciseType::Resistance, Some("Legs"))?;

    // 1. Create Alias
    service.create_alias("bp", "Barbell Bench Press")?;

    // 2. List Aliases
    let aliases = service.list_aliases()?;
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases.get("bp").unwrap(), "Barbell Bench Press");

    // 3. Resolve Alias
    let resolved_def = service.resolve_exercise_identifier("bp")?.unwrap();
    assert_eq!(resolved_def.name, "Barbell Bench Press");
    assert_eq!(resolved_def.id, ex_id);

    // 4. Try creating duplicate alias
    let result = service.create_alias("bp", "Squat"); // Different exercise, same alias
    assert!(result.is_err());
    // println!("{:?}",result); // Keep for debugging if needed
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
    let today = Utc::now().date_naive();
    let (workout_id, _) = service.add_workout(
        "bp",
        today,
        Some(3),
        Some(5),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    let workouts = service.list_workouts(WorkoutFilters {
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

// Test for editing exercise name and having aliases/workouts update
#[test]
fn test_edit_exercise_with_alias_and_name_change() -> Result<()> {
    let mut service = create_test_service()?; // Service owns its connection

    service.create_exercise("Old Name", ExerciseType::Resistance, Some("muscle1"))?;
    service.create_alias("on", "Old Name")?;

    // Add a workout using the alias
    let today = Utc::now().date_naive();
    service.add_workout(
        "on",
        today,
        Some(1),
        Some(1),
        Some(1.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Edit using the alias identifier, changing the name and muscles
    service.edit_exercise(
        "on", // Identify by alias
        Some("New Name"),
        None,                          // Keep type
        Some(Some("muscle1,muscle2")), // Change muscles
    )?;

    // --- Verification using the same service instance ---

    // 1. Check old name resolves to None
    assert!(service.resolve_exercise_identifier("Old Name")?.is_none());

    // 2. Check alias now points to the new definition
    let resolved_by_alias = service
        .resolve_exercise_identifier("on")?
        .expect("Alias 'on' should resolve");
    assert_eq!(resolved_by_alias.name, "New Name");
    assert_eq!(
        resolved_by_alias.muscles,
        Some("muscle1,muscle2".to_string())
    );

    // 3. Check new name resolves correctly
    let resolved_by_new_name = service
        .resolve_exercise_identifier("New Name")?
        .expect("'New Name' should resolve");
    assert_eq!(resolved_by_new_name.id, resolved_by_alias.id); // Ensure same exercise ID
    assert_eq!(resolved_by_new_name.name, "New Name");
    assert_eq!(
        resolved_by_new_name.muscles,
        Some("muscle1,muscle2".to_string())
    );

    // 4. Check the alias list still contains the alias pointing to the NEW name
    let aliases = service.list_aliases()?;
    assert_eq!(
        aliases.get("on").expect("Alias 'on' should exist"),
        "New Name"
    );

    // 5. Check the workout entry associated with the old name was updated
    //    List workouts using the *alias* which should now resolve to "New Name"
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("on"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1, "Should find one workout via alias");
    assert_eq!(
        workouts[0].exercise_name, "New Name",
        "Workout exercise name should be updated"
    );

    //    List workouts using the *new name*
    let workouts_new_name = service.list_workouts(WorkoutFilters {
        exercise_name: Some("New Name"),
        ..Default::default()
    })?;
    assert_eq!(
        workouts_new_name.len(),
        1,
        "Should find one workout via new name"
    );
    assert_eq!(
        workouts_new_name[0].id, workouts[0].id,
        "Workout IDs should match"
    );

    //    List workouts using the *old name* (should find none)
    let workouts_old_name = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Old Name"),
        ..Default::default()
    });
    // Expect ExerciseNotFound error because the filter tries to resolve "Old Name" which no longer exists
    assert!(workouts_old_name.is_err());
    assert!(workouts_old_name
        .unwrap_err()
        .downcast_ref::<DbError>()
        .map_or(false, |e| matches!(e, DbError::ExerciseNotFound(_))));

    Ok(())
}

#[test]
fn test_delete_exercise_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("To Delete", ExerciseType::Cardio, None)?;
    service.create_alias("td", "To Delete")?;

    // Delete exercise using alias
    let result = service.delete_exercise(&vec!["td".to_string()])?;
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
    service.create_exercise("Rowing", ExerciseType::Cardio, None)?;

    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let two_days_ago = Utc::now().date_naive() - Duration::days(2);

    service.add_workout(
        "Rowing",
        yesterday,
        None,
        None,
        None,
        Some(30),
        None,
        None,
        None,
        None,
        None,
    )?;
    service.add_workout(
        "Rowing",
        two_days_ago,
        None,
        None,
        None,
        Some(25),
        None,
        None,
        None,
        None,
        None,
    )?;

    // List for yesterday
    let workouts_yesterday = service.list_workouts(WorkoutFilters {
        date: Some(yesterday),
        ..Default::default()
    })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].duration_minutes, Some(30));
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);

    // List for two days ago
    let workouts_two_days_ago = service.list_workouts(WorkoutFilters {
        date: Some(two_days_ago),
        ..Default::default()
    })?;
    assert_eq!(workouts_two_days_ago.len(), 1);
    assert_eq!(workouts_two_days_ago[0].duration_minutes, Some(25));
    assert_eq!(
        workouts_two_days_ago[0].timestamp.date_naive(),
        two_days_ago
    );

    Ok(())
}

#[test]
fn test_edit_workout_date() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Push-ups", ExerciseType::BodyWeight, None)?;
    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);

    let (workout_id, _) = service.add_workout(
        "Push-ups",
        today,
        Some(3),
        Some(15),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(70.0),
    )?;

    // Edit the date
    service.edit_workout(
        workout_id,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(yesterday),
    )?;

    // Verify date change by listing
    let workouts_today = service.list_workouts(WorkoutFilters {
        date: Some(today),
        ..Default::default()
    })?;
    assert!(workouts_today.is_empty());

    let workouts_yesterday = service.list_workouts(WorkoutFilters {
        date: Some(yesterday),
        ..Default::default()
    })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].id, workout_id);
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);

    Ok(())
}

// TODO
// Updated PB test to reflect new PBInfo structure and separate config flags
// #[test]
// fn test_pb_detection_and_config() -> Result<()> {
//     let mut service = create_test_service()?; // Uses default config with all PBs enabled
//     service.create_exercise("Deadlift", ExerciseType::Resistance, Some("back,legs"))?;
//     service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
//     let today = Utc::now().date_naive();

//     // --- Test Default Config (All PBs Enabled) ---

//     // Workout 1: Baseline
//     let (_, pb1) = service.add_workout("Deadlift", today, Some(1), Some(5), Some(100.0), None, None, None, None, None, None)?;
//     assert!(pb1.is_none(), "First workout shouldn't be a PB");
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 2: Weight PB
//     let (_, pb2) = service.add_workout("Deadlift", today, Some(1), Some(3), Some(110.0), None, None, None, None, None, None)?;
//     assert!(pb2.is_some(), "Should detect weight PB");
//     let info2 = pb2.unwrap();
//     assert!(info2.achieved_weight_pb);
//     assert!(!info2.achieved_reps_pb);
//     assert_eq!(info2.new_weight, Some(110.0));
//     assert_eq!(info2.previous_weight, Some(100.0)); // Previous was 100
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 3: Reps PB
//     let (_, pb3) = service.add_workout("Deadlift", today, Some(3), Some(6), Some(90.0), None, None, None, None, None, None)?;
//     assert!(pb3.is_some(), "Should detect reps PB");
//     let info3 = pb3.unwrap();
//     assert!(!info3.achieved_weight_pb);
//     assert!(info3.achieved_reps_pb);
//     assert_eq!(info3.new_reps, Some(6));
//     assert_eq!(info3.previous_reps, Some(5)); // Previous was 5
//     thread::sleep(StdDuration::from_millis(10));

//      // Workout 4: Both Weight and Reps PB
//     let (_, pb4) = service.add_workout("Deadlift", today, Some(1), Some(7), Some(120.0), None, None, None, None, None, None)?;
//     assert!(pb4.is_some(), "Should detect both PBs");
//     let info4 = pb4.unwrap();
//     assert!(info4.achieved_weight_pb);
//     assert!(info4.achieved_reps_pb);
//     assert_eq!(info4.new_weight, Some(120.0));
//     assert_eq!(info4.previous_weight, Some(110.0)); // Previous was 110
//     assert_eq!(info4.new_reps, Some(7));
//     assert_eq!(info4.previous_reps, Some(6)); // Previous was 6
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 5: No PB
//     let (_, pb5) = service.add_workout("Deadlift", today, Some(5), Some(5), Some(105.0), None, None, None, None, None, None)?;
//     assert!(pb5.is_none(), "Should not detect PB");
//     thread::sleep(StdDuration::from_millis(10));

//     // --- Test Disabling Specific PBs ---
//     service.set_pb_notify_reps(false)?; // Disable Rep PB notifications

//     // Workout 6: Weight PB (should be detected)
//     let (_, pb6) = service.add_workout("Deadlift", today, Some(1), Some(4), Some(130.0), None, None, None, None, None, None)?;
//     assert!(pb6.is_some(), "Weight PB should still be detected");
//     assert!(pb6.unwrap().achieved_weight_pb);
//     thread::sleep(StdDuration::from_millis(10));

//     // Workout 7: Reps PB (should NOT be detected as PB *notification*)
//     let (_, pb7) = service.add_workout("Deadlift", today, Some(1), Some(8), Some(125.0), None, None, None, None, None, None)?;
//     assert!(pb7.is_none(), "Reps PB should NOT trigger notification when disabled");
//     thread::sleep(StdDuration::from_millis(10));

//     // --- Test Duration/Distance PBs ---
//     service.set_pb_notify_reps(true)?; // Re-enable reps
//     service.set_pb_notify_weight(false)?; // Disable weight

//     // Running Workout 1: Baseline
//      let (_, rpb1) = service.add_workout("Running", today, None, None, None, Some(30), Some(5.0), None, None, None, None)?; // 5km in 30min
//      assert!(rpb1.is_none());
//      thread::sleep(StdDuration::from_millis(10));

//      // Running Workout 2: Duration PB (longer duration, same distance)
//      let (_, rpb2) = service.add_workout("Running", today, None, None, None, Some(35), Some(5.0), None, None, None, None)?;
//      assert!(rpb2.is_some());
//      let rinfo2 = rpb2.unwrap();
//      assert!(rinfo2.achieved_duration_pb);
//      assert!(!rinfo2.achieved_distance_pb);
//      assert_eq!(rinfo2.new_duration, Some(35));
//      assert_eq!(rinfo2.previous_duration, Some(30));
//      thread::sleep(StdDuration::from_millis(10));

//       // Running Workout 3: Distance PB (longer distance, irrelevant duration)
//      let (_, rpb3) = service.add_workout("Running", today, None, None, None, Some(25), Some(6.0), None, None, None, None)?;
//      assert!(rpb3.is_some());
//      let rinfo3 = rpb3.unwrap();
//      assert!(!rinfo3.achieved_duration_pb);
//      assert!(rinfo3.achieved_distance_pb);
//      assert_eq!(rinfo3.new_distance, Some(6.0));
//      assert_eq!(rinfo3.previous_distance, Some(5.0));
//      thread::sleep(StdDuration::from_millis(10));

//      // Running Workout 4: Disable distance PB, achieve distance PB -> No notification
//      service.set_pb_notify_distance(false)?;
//      let (_, rpb4) = service.add_workout("Running", today, None, None, None, Some(40), Some(7.0), None, None, None, None)?;
//      assert!(rpb4.is_some(), "Duration PB should still trigger"); // Duration PB is still active
//      let rinfo4 = rpb4.unwrap();
//      assert!(rinfo4.achieved_duration_pb);
//      assert!(!rinfo4.achieved_distance_pb, "Distance PB flag should be false in returned info if notify disabled"); // Flag should reflect config state at time of adding

//     Ok(())
// }

#[test]
fn test_create_and_list_exercises() -> Result<()> {
    let service = create_test_service()?;

    // Create some exercises
    service.create_exercise(
        "Bench Press",
        ExerciseType::Resistance,
        Some("chest,triceps"),
    )?;
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back,biceps"))?;

    // List all exercises
    let exercises = service.list_exercises(None, None)?;
    assert_eq!(exercises.len(), 3);

    // Filter by type
    let resistance_exercises = service.list_exercises(Some(ExerciseType::Resistance), None)?;
    assert_eq!(resistance_exercises.len(), 1);
    assert_eq!(resistance_exercises[0].name, "Bench Press");

    // Filter by muscle
    let leg_exercises = service.list_exercises(None, Some("legs"))?;
    assert_eq!(leg_exercises.len(), 1);
    assert_eq!(leg_exercises[0].name, "Running");

    Ok(())
}

#[test]
fn test_pb_config_interaction() -> Result<()> {
    let mut service = create_test_service()?; // PB notifications default to Some(true) here

    // Check initial state (global enabled)
    assert_eq!(service.check_pb_notification_config()?, true);

    // Disable PB notifications globally
    service.set_pb_notification_enabled(false)?;
    assert_eq!(service.config.notify_pb_enabled, Some(false));
    assert_eq!(service.check_pb_notification_config()?, false);

    // Re-enable PB notifications globally
    service.set_pb_notification_enabled(true)?;
    assert_eq!(service.config.notify_pb_enabled, Some(true));
    assert_eq!(service.check_pb_notification_config()?, true);

    // Test case where config starts as None (simulate first run)
    service.config.notify_pb_enabled = None;
    let result = service.check_pb_notification_config();
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::PbNotificationNotSet => {} // Correct error
        _ => panic!("Expected PbNotificationNotSet error"),
    }

    // Test individual metric flags
    assert!(service.config.notify_pb_weight); // Should be true by default
    service.set_pb_notify_weight(false)?;
    assert!(!service.config.notify_pb_weight);
    service.set_pb_notify_weight(true)?;
    assert!(service.config.notify_pb_weight);

    assert!(service.config.notify_pb_reps);
    service.set_pb_notify_reps(false)?;
    assert!(!service.config.notify_pb_reps);

    assert!(service.config.notify_pb_duration);
    service.set_pb_notify_duration(false)?;
    assert!(!service.config.notify_pb_duration);

    assert!(service.config.notify_pb_distance);
    service.set_pb_notify_distance(false)?;
    assert!(!service.config.notify_pb_distance);

    Ok(())
}

// Test list filtering with aliases
#[test]
fn test_list_filter_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise(
        "Overhead Press",
        ExerciseType::Resistance,
        Some("shoulders"),
    )?;
    service.create_alias("ohp", "Overhead Press")?;
    let today = Utc::now().date_naive();

    service.add_workout(
        "ohp",
        today,
        Some(5),
        Some(5),
        Some(50.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Filter list using alias
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("ohp"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].exercise_name, "Overhead Press");

    // Filter list using canonical name
    let workouts2 = service.list_workouts(WorkoutFilters {
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

    // Create an exercise first
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;

    // Add some workouts
    let date1 = NaiveDate::from_ymd_opt(2015, 6, 2).unwrap();
    service.add_workout(
        "Running",
        date1,
        None,
        None,
        None,
        Some(30),
        Some(5.0), // 5 km
        Some("First run".to_string()),
        None,
        None,
        None,
    )?;

    let date2 = NaiveDate::from_ymd_opt(2015, 6, 3).unwrap();
    service.add_workout(
        "Running",
        date2,
        None,
        None,
        None,
        Some(60),
        Some(10.5), // 10.5 km
        Some("Second run".to_string()),
        None,
        None,
        None,
    )?;

    // List workouts
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Running"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 2);
    // Most recent first
    assert_eq!(workouts[0].timestamp.date_naive(), date2);
    assert_eq!(workouts[0].duration_minutes, Some(60));
    assert_eq!(workouts[0].distance, Some(10.5)); // Check distance stored (km)

    assert_eq!(workouts[1].timestamp.date_naive(), date1);
    assert_eq!(workouts[1].duration_minutes, Some(30));
    assert_eq!(workouts[1].distance, Some(5.0));

    Ok(())
}

#[test]
fn test_add_workout_imperial_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Imperial; // Set units to Imperial

    service.create_exercise("Cycling", ExerciseType::Cardio, None)?;
    let today = Utc::now().date_naive();

    let miles_input = 10.0;
    let expected_km = miles_input * 1.60934;

    service.add_workout(
        "Cycling",
        today,
        None,
        None,
        None,
        Some(45),
        Some(miles_input),
        None,
        None,
        None,
        None,
    )?;

    // List workout and check stored distance (should be km)
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Cycling"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert!(workouts[0].distance.is_some());
    // Compare with tolerance for floating point
    assert!((workouts[0].distance.unwrap() - expected_km).abs() < 0.001);

    Ok(())
}

#[test]
fn test_edit_workout_distance() -> Result<()> {
    let mut service = create_test_service()?;
    service.config.units = Units::Metric;
    service.create_exercise("Walking", ExerciseType::Cardio, None)?;
    let today = Utc::now().date_naive();

    let (workout_id, _) = service.add_workout(
        "Walking",
        today,
        None,
        None,
        None,
        Some(60),
        Some(5.0),
        None,
        None,
        None,
        None,
    )?;

    // Edit distance
    let new_distance = 7.5;
    service.edit_workout(
        workout_id,
        None,
        None,
        None,
        None,
        None,
        Some(new_distance),
        None,
        None,
    )?;

    // Verify
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Walking"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, workout_id);
    assert_eq!(workouts[0].distance, Some(new_distance));

    // Edit distance with Imperial units input
    service.config.units = Units::Imperial;
    let imperial_input = 2.0; // 2 miles
    let expected_km_edit = imperial_input * 1.60934;
    service.edit_workout(
        workout_id,
        None,
        None,
        None,
        None,
        None,
        Some(imperial_input),
        None,
        None,
    )?;

    // Verify stored km
    let workouts_imperial_edit = service.list_workouts(WorkoutFilters {
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
    service.config.bodyweight = Some(70.0); // Set bodyweight

    // Create a bodyweight exercise
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back"))?;

    // Add workout with additional weight
    service.add_workout(
        "Pull-ups",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        Some(3),
        Some(10),
        Some(5.0), // Additional weight
        None,
        None, // No distance
        None,
        None,
        None,
        Some(70.0), // Pass bodyweight
    )?;

    // Check that weight was calculated correctly
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Pull-ups"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].weight, Some(75.0)); // 70 + 5

    Ok(())
}

#[test]
fn test_edit_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Edit the exercise
    service.edit_exercise(
        "Bench Press",
        Some("Barbell Bench Press"),
        Some(ExerciseType::Resistance),
        Some(Some("chest,triceps,shoulders")),
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
    let edit_result = service.edit_exercise("NonExistent", Some("WontWork"), None, None);
    assert!(edit_result.is_err());
    assert!(edit_result
        .unwrap_err()
        .downcast_ref::<DbError>()
        .map_or(false, |e| matches!(e, DbError::ExerciseNotFound(_))));

    Ok(())
}

#[test]
fn test_delete_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Delete it
    let result = service.delete_exercise(&vec!["Bench Press".to_string()])?;
    assert_eq!(result, 1);

    // Verify it's gone
    let exercise = service.get_exercise_by_identifier_service("Bench Press")?;
    assert!(exercise.is_none());

    // Try deleting non-existent exercise
    let delete_result = service.delete_exercise(&vec!["NonExistent".to_string()]);
    assert!(delete_result.is_err());
    assert!(delete_result
        .unwrap_err()
        .downcast_ref::<DbError>()
        .map_or(false, |e| matches!(e, DbError::ExerciseNotFound(_))));

    Ok(())
}

#[test]
fn test_workout_filters() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Curl", ExerciseType::Resistance, Some("Biceps"))?;

    // Add workouts on different dates
    // Hack: We can't set the timestamp directly, so we'll add with a small delay
    let date1 = NaiveDate::from_ymd_opt(2015, 6, 3).unwrap();
    service.add_workout(
        "Running",
        date1,
        None,
        None,
        None,
        Some(30),
        Some(5.0),
        None,
        None,
        None,
        None,
    )?;
    thread::sleep(StdDuration::from_millis(10)); // Ensure different timestamp

    // Add another workout on same date but later time
    service.add_workout(
        "Curl",
        date1,
        Some(3),
        Some(12),
        Some(15.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Filter by type
    let resistance_workouts = service.list_workouts(WorkoutFilters {
        exercise_type: Some(ExerciseType::Resistance),
        ..Default::default()
    })?;
    assert_eq!(resistance_workouts.len(), 1);
    assert_eq!(resistance_workouts[0].exercise_name, "Curl");
    assert_eq!(resistance_workouts[0].reps, Some(12));

    let cardio_workouts = service.list_workouts(WorkoutFilters {
        exercise_type: Some(ExerciseType::Cardio),
        ..Default::default()
    })?;
    assert_eq!(cardio_workouts.len(), 1);
    assert_eq!(cardio_workouts[0].exercise_name, "Running");
    assert_eq!(cardio_workouts[0].distance, Some(5.0));

    // Filter by date
    let date1_workouts = service.list_workouts(WorkoutFilters {
        date: Some(date1),
        ..Default::default()
    })?;
    assert_eq!(date1_workouts.len(), 2); // Both workouts were on this date

    Ok(())
}

#[test]
fn test_nth_last_day_workouts() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Squats", ExerciseType::Resistance, Some("legs"))?;

    // Add workouts on different dates
    let date1 = NaiveDate::from_ymd_opt(2015, 6, 2).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2015, 6, 7).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2015, 6, 9).unwrap(); // Most recent

    // Workout 1 (oldest)
    service.add_workout(
        "Squats",
        date1,
        Some(3),
        Some(10),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    // Workout 2 (middle)
    service.add_workout(
        "Squats",
        date2,
        Some(5),
        Some(5),
        Some(120.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    // Workout 3 (most recent)
    service.add_workout(
        "Squats",
        date3,
        Some(4),
        Some(6),
        Some(125.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Get workouts for the most recent day (n=1)
    let recent_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 1)?;
    assert_eq!(recent_workouts.len(), 1);
    assert_eq!(recent_workouts[0].sets, Some(4));
    assert_eq!(recent_workouts[0].timestamp.date_naive(), date3);

    // Get workouts for the second most recent day (n=2)
    let previous_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 2)?;
    assert_eq!(previous_workouts.len(), 1);
    assert_eq!(previous_workouts[0].sets, Some(5));
    assert_eq!(previous_workouts[0].timestamp.date_naive(), date2);

    // Get workouts for the third most recent day (n=3)
    let oldest_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 3)?;
    assert_eq!(oldest_workouts.len(), 1);
    assert_eq!(oldest_workouts[0].sets, Some(3));
    assert_eq!(oldest_workouts[0].timestamp.date_naive(), date1);

    // Try getting n=4 (should be empty)
    let no_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 4)?;
    assert!(no_workouts.is_empty());

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
    assert_eq!(service.config.streak_interval_days, 1);
    service.set_streak_interval(3)?;
    assert_eq!(service.config.streak_interval_days, 3);
    let interval_result = service.set_streak_interval(0); // Test invalid interval
    assert!(interval_result.is_err()); // Should fail

    Ok(())
}

#[test]
fn test_exercise_not_found() -> Result<()> {
    let mut service = create_test_service()?;

    // Try to get non-existent exercise
    let result = service.get_exercise_by_identifier_service("Non-existent");
    assert!(result.is_ok()); // Should return Ok(None)
    assert!(result?.is_none());

    // Try to edit non-existent exercise
    let result = service.edit_exercise("Non-existent", None, None, None);
    assert!(result.is_err());
    match result.unwrap_err().downcast_ref::<DbError>() {
        Some(DbError::ExerciseNotFound(_)) => (),
        _ => panic!("Expected ExerciseNotFound error"),
    }

    // Try to delete non-existent exercise
    let result = service.delete_exercise(&vec!["Non-existent".to_string()]);
    assert!(result.is_err());
    match result.unwrap_err().downcast_ref::<DbError>() {
        Some(DbError::ExerciseNotFound(_)) => (),
        _ => panic!("Expected ExerciseNotFound error"),
    }

    // Try to add workout for non-existent exercise without implicit details
    let result = service.add_workout(
        "Non-existent",
        Utc::now().date_naive(),
        Some(1),
        Some(1),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found. Define it first"));

    Ok(())
}

#[test]
fn test_workout_not_found() -> Result<()> {
    let service = create_test_service()?;

    // Try to edit non-existent workout
    let result = service.edit_workout(999, None, None, None, None, None, None, None, None);
    assert!(result.is_err());
    // match result.unwrap_err().downcast_ref::<DbError>() {
    //      Some(DbError::WorkoutNotFound(999)) => (), // Correct error and ID
    //      _ => panic!("Expected WorkoutNotFound error with ID 999"),
    // }

    // Try to delete non-existent workout
    let result = service.delete_workouts(&vec![999]);
    assert!(result.is_err());
    //  match result.unwrap_err().downcast_ref::<DbError>() {
    //     Some(DbError::WorkoutNotFound(999)) => (), // Correct error and ID
    //     _ => panic!("Expected WorkoutNotFound error with ID 999"),
    // }

    Ok(())
}

#[test]
fn test_bodyweight_validation() -> Result<()> {
    let mut service = create_test_service()?;

    // Test invalid bodyweight
    let result = service.set_bodyweight(0.0);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    let result = service.set_bodyweight(-10.0);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    Ok(())
}

#[test]
fn test_set_units() -> Result<()> {
    let mut service = create_test_service()?;
    assert_eq!(service.config.units, Units::Metric);

    // Set to Imperial
    service.set_units(Units::Imperial)?;
    assert_eq!(service.config.units, Units::Imperial);

    // Set back to Metric
    service.set_units(Units::Metric)?;
    assert_eq!(service.config.units, Units::Metric);

    Ok(())
}

// Test Workout Volume Calculation (Feature 1)
#[test]
fn test_workout_volume() -> Result<()> {
    let mut service = create_test_service()?;
    let day1 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
    let day2 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();

    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back"))?; // BW with added weight
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;
    service.create_exercise("Squats", ExerciseType::Resistance, Some("legs"))?;

    // Day 1 Workouts
    service.add_workout(
        "Bench Press",
        day1,
        Some(3),
        Some(10),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 3*10*100 = 3000
    service.add_workout(
        "Bench Press",
        day1,
        Some(1),
        Some(8),
        Some(105.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 1*8*105 = 840
    service.add_workout(
        "Pull-ups",
        day1,
        Some(4),
        Some(6),
        Some(10.0),
        None,
        None,
        None,
        None,
        None,
        Some(70.0),
    )?; // Vol = 4*6*(70+10) = 1920
    service.add_workout(
        "Running",
        day1,
        None,
        None,
        None,
        Some(30),
        Some(5.0),
        None,
        None,
        None,
        None,
    )?; // Vol = 0

    // Day 2 Workouts
    service.add_workout(
        "Squats",
        day2,
        Some(5),
        Some(5),
        Some(120.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 5*5*120 = 3000
    service.add_workout(
        "Bench Press",
        day2,
        Some(4),
        Some(6),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // Vol = 4*6*100 = 2400

    // --- Test Volume Calculation ---

    // Total volume per day/exercise (no filters, default limit)
    let volume_all = service.calculate_daily_volume(VolumeFilters::default())?;
    // Should return rows per exercise per day, most recent day first
    // Expected: (day2, Squats, 3000), (day2, Bench Press, 2400), (day1, Bench Press, 3840), (day1, Pull-ups, 1920), (day1, Running, 0)
    assert_eq!(volume_all.len(), 5);

    // Verify Day 2 data (order within day is by name ASC)
    assert_eq!(volume_all[0].0, day2);
    assert_eq!(volume_all[0].1, "Bench Press"); // BP comes before Squats alphabetically
    assert!((volume_all[0].2 - 2400.0).abs() < 0.01);
    assert_eq!(volume_all[1].0, day2);
    assert_eq!(volume_all[1].1, "Squats");
    assert!((volume_all[1].2 - 3000.0).abs() < 0.01);

    // Verify Day 1 data (order within day is by name ASC)
    assert_eq!(volume_all[2].0, day1);
    assert_eq!(volume_all[2].1, "Bench Press"); // BP
    assert!((volume_all[2].2 - (3000.0 + 840.0)).abs() < 0.01); // 3840
    assert_eq!(volume_all[3].0, day1);
    assert_eq!(volume_all[3].1, "Pull-ups"); // Pull-ups
    assert!((volume_all[3].2 - 1920.0).abs() < 0.01);
    assert_eq!(volume_all[4].0, day1);
    assert_eq!(volume_all[4].1, "Running"); // Running
    assert!((volume_all[4].2 - 0.0).abs() < 0.01);

    // Volume for Day 1 only
    let volume_day1 = service.calculate_daily_volume(VolumeFilters {
        start_date: Some(day1),
        end_date: Some(day1),
        ..Default::default()
    })?;
    assert_eq!(volume_day1.len(), 3); // BP, Pull-ups, Running on day 1
                                      // Could check specific values if needed

    // Volume for "Bench Press" only
    let volume_bp = service.calculate_daily_volume(VolumeFilters {
        exercise_name: Some("Bench Press"),
        ..Default::default()
    })?;
    assert_eq!(volume_bp.len(), 2); // BP on day 1 and day 2
                                    // Day 2 BP
    assert_eq!(volume_bp[0].0, day2);
    assert_eq!(volume_bp[0].1, "Bench Press");
    assert!((volume_bp[0].2 - 2400.0).abs() < 0.01);
    // Day 1 BP
    assert_eq!(volume_bp[1].0, day1);
    assert_eq!(volume_bp[1].1, "Bench Press");
    assert!((volume_bp[1].2 - 3840.0).abs() < 0.01); // Sum of both BP entries

    // Volume for Cardio (should be 0)
    let volume_cardio = service.calculate_daily_volume(VolumeFilters {
        exercise_type: Some(ExerciseType::Cardio),
        ..Default::default()
    })?;
    assert_eq!(volume_cardio.len(), 1); // Only Running on day 1
    assert_eq!(volume_cardio[0].0, day1);
    assert_eq!(volume_cardio[0].1, "Running");
    assert_eq!(volume_cardio[0].2, 0.0);

    Ok(())
}

// Test Exercise Statistics Calculation
#[test]
fn test_exercise_stats() -> Result<()> {
    let mut service = create_test_service()?;
    let day1 = NaiveDate::from_ymd_opt(2023, 10, 20).unwrap(); // Fri
    let day2 = NaiveDate::from_ymd_opt(2023, 10, 22).unwrap(); // Sun (Gap 1 day -> 2 days total)
    let day3 = NaiveDate::from_ymd_opt(2023, 10, 23).unwrap(); // Mon (Gap 0 days -> 1 day total)
    let day4 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(); // Fri (Gap 3 days -> 4 days total) - Longest Gap 3
    let day5 = NaiveDate::from_ymd_opt(2023, 10, 28).unwrap(); // Sat (Gap 0 days -> 1 day total)

    service.create_exercise("Test Stats", ExerciseType::Resistance, None)?;

    // Add workouts
    service.add_workout(
        "Test Stats",
        day1,
        Some(3),
        Some(10),
        Some(50.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // PB: W=50, R=10
    service.add_workout(
        "Test Stats",
        day2,
        Some(3),
        Some(8),
        Some(55.0),
        Some(10),
        None,
        None,
        None,
        None,
        None,
    )?; // PB: W=55, D=10
    service.add_workout(
        "Test Stats",
        day3,
        Some(4),
        Some(6),
        Some(50.0),
        Some(12),
        None,
        None,
        None,
        None,
        None,
    )?; // PB: D=12
    service.add_workout(
        "Test Stats",
        day4,
        Some(2),
        Some(12),
        Some(45.0),
        None,
        Some(5.0),
        None,
        None,
        None,
        None,
    )?; // PB: R=12, Dist=5.0
    service.add_workout(
        "Test Stats",
        day5,
        Some(3),
        Some(10),
        Some(55.0),
        Some(10),
        Some(5.5),
        None,
        None,
        None,
        None,
    )?; // PB: Dist=5.5

    // --- Test with daily streak interval (default) ---
    let stats_daily = service.get_exercise_stats("Test Stats")?;

    assert_eq!(stats_daily.canonical_name, "Test Stats");
    assert_eq!(stats_daily.total_workouts, 5);
    assert_eq!(stats_daily.first_workout_date, Some(day1));
    assert_eq!(stats_daily.last_workout_date, Some(day5));

    // Avg/week: 5 workouts / (8 days / 7 days/week) = 5 / (8/7) = 35/8 = 4.375
    assert!((stats_daily.avg_workouts_per_week.unwrap() - 4.375).abs() < 0.01);

    // Gaps: day2-day1=2 days, day3-day2=1 day, day4-day3=4 days, day5-day4=1 day. Max gap = 4 days. DB stores diff, we want gap between.
    // Longest gap days: (day2-day1)-1 = 1, (day3-day2)-1 = 0, (day4-day3)-1 = 3, (day5-day4)-1 = 0 -> Max = 3
    assert_eq!(stats_daily.longest_gap_days, Some(3));

    // Streaks (daily interval = 1 day):
    // day1 -> day2 (gap 1) NO
    // day2 -> day3 (gap 0) YES (Streak: day2, day3 = 2)
    // day3 -> day4 (gap 3) NO
    // day4 -> day5 (gap 0) YES (Streak: day4, day5 = 2)
    // Longest = 2. Current = 2 (ends on day5, today is > day5) -> Should current be 0? Yes.
    assert_eq!(stats_daily.current_streak, 0); // Assuming test runs after day5
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
    // day1 -> day2 (gap 1 <= 2) YES (Streak: day1, day2 = 2)
    // day2 -> day3 (gap 0 <= 2) YES (Streak: day1, day2, day3 = 3)
    // day3 -> day4 (gap 3 > 2) NO
    // day4 -> day5 (gap 0 <= 2) YES (Streak: day4, day5 = 2)
    // Longest = 3. Current = 0 (ends on day5, today > day5+2)
    assert_eq!(stats_2day.current_streak, 0);
    assert_eq!(stats_2day.longest_streak, 3);

    // --- Test Edge Cases ---
    // Test stats for exercise with no workouts
    service.create_exercise("No Workouts", ExerciseType::Cardio, None)?;
    let no_workout_result = service.get_exercise_stats("No Workouts");
    assert!(no_workout_result.is_err());
    match no_workout_result.unwrap_err().downcast_ref::<DbError>() {
        Some(DbError::NoWorkoutDataFound(_)) => (),
        _ => panic!("Expected NoWorkoutDataFound error"),
    }

    // Test stats for exercise with one workout
    service.create_exercise("One Workout", ExerciseType::Resistance, None)?;
    let day_single = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();
    service.add_workout(
        "One Workout",
        day_single,
        Some(1),
        Some(5),
        Some(10.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    let one_workout_stats = service.get_exercise_stats("One Workout")?;

    assert_eq!(one_workout_stats.total_workouts, 1);
    assert_eq!(one_workout_stats.first_workout_date, Some(day_single));
    assert_eq!(one_workout_stats.last_workout_date, Some(day_single));
    assert!(one_workout_stats.avg_workouts_per_week.is_none());
    assert!(one_workout_stats.longest_gap_days.is_none());
    assert_eq!(one_workout_stats.current_streak, 0); // Needs >= 1 day gap from today
    assert_eq!(one_workout_stats.longest_streak, 1);
    assert_eq!(one_workout_stats.personal_bests.max_weight, Some(10.0));
    assert_eq!(one_workout_stats.personal_bests.max_reps, Some(5));
    assert!(one_workout_stats
        .personal_bests
        .max_duration_minutes
        .is_none());
    assert!(one_workout_stats.personal_bests.max_distance_km.is_none());

    Ok(())
}

#[test]
fn test_get_latest_bodyweight() -> Result<()> {
    let service = create_test_service()?;

    // Test when empty
    assert!(service.get_latest_bodyweight()?.is_none());

    // Add some entries
    service.add_bodyweight_entry(Utc::now() - Duration::days(2), 70.0)?;
    service.add_bodyweight_entry(Utc::now() - Duration::days(1), 71.0)?; // Latest

    // Get latest
    let latest = service.get_latest_bodyweight()?;
    assert!(latest.is_some());
    assert_eq!(latest.unwrap(), 71.0);

    Ok(())
}

#[test]
fn test_delete_body_weight() -> Result<()> {
    let mut service = create_test_service()?;
    let id = service.add_bodyweight_entry(Utc::now() - Duration::days(2), 50.0)?;
    let result = service.delete_bodyweight(1);
    assert!(result.is_ok());
    let result = service.delete_bodyweight(1);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_bodyweight_workout_needs_log() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back"))?;

    // Try adding BW workout *before* logging bodyweight
    // This test now relies on the logic in main.rs to fetch the BW.
    // We simulate that check here.
    let latest_bw = service.get_latest_bodyweight()?;
    assert!(latest_bw.is_none()); // Should be none initially

    // If main.rs were running, it would get None and bail.
    // We can simulate the direct call to add_workout with None for bodyweight_to_use
    // which should now trigger the internal error check.
    let result = service.add_workout(
        "Pull-ups",
        Utc::now().date_naive(),
        Some(3),
        Some(5),
        Some(10.0), // Additional weight
        None,
        None,
        None,
        None,
        None,
        None, // Simulate not finding a logged bodyweight
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Bodyweight is required"));

    // Now log bodyweight and try again
    service.add_bodyweight_entry(Utc::now(), 75.0)?;
    let logged_bw = service.get_latest_bodyweight()?.unwrap();

    // Simulate main.rs fetching BW and passing it
    let add_result = service.add_workout(
        "Pull-ups",
        Utc::now().date_naive(),
        Some(3),
        Some(5),
        Some(10.0), // Additional weight
        None,
        None,
        None,
        None,
        None,
        Some(logged_bw), // Pass the fetched BW
    );

    assert!(add_result.is_ok());
    let (id, _) = add_result?;

    // Verify workout weight was calculated correctly
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Pull-ups"),
        ..Default::default()
    })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].id, id);
    assert_eq!(workouts[0].weight, Some(85.0)); // 75.0 (logged) + 10.0 (additional)

    Ok(())
}

#[test]
fn test_add_list_bodyweight() -> Result<()> {
    let service = create_test_service()?;
    let date1 = Utc::now() - Duration::days(2);
    let date2 = Utc::now() - Duration::days(1);
    let date3 = Utc::now();

    // Add entries
    service.add_bodyweight_entry(date1, 70.5)?;
    service.add_bodyweight_entry(date2, 71.0)?;
    service.add_bodyweight_entry(date3, 70.8)?;

    // List entries (default limit should be high enough)
    let entries = service.list_bodyweights(10)?;
    assert_eq!(entries.len(), 3);

    // Check order (descending by timestamp)
    assert_eq!(entries[0].2, 70.8); // date3
                                    // Tolerate small difference in timestamp comparison
    assert!((entries[0].1 - date3).num_milliseconds().abs() < 100);

    assert_eq!(entries[1].2, 71.0); // date2
    assert!((entries[1].1 - date2).num_milliseconds().abs() < 100);

    assert_eq!(entries[2].2, 70.5); // date1
    assert!((entries[2].1 - date1).num_milliseconds().abs() < 100);

    // Test limit
    let limited_entries = service.list_bodyweights(1)?;
    assert_eq!(limited_entries.len(), 1);
    assert_eq!(limited_entries[0].2, 70.8); // Should be the latest one

    Ok(())
}

#[test]
fn test_target_bodyweight_config() -> Result<()> {
    let mut service = create_test_service()?;

    // Initially None
    assert!(service.get_target_bodyweight().is_none());

    // Set a target
    service.set_target_bodyweight(Some(78.5))?;
    assert_eq!(service.config.target_bodyweight, Some(78.5));
    assert_eq!(service.get_target_bodyweight(), Some(78.5));

    // Set another target
    service.set_target_bodyweight(Some(77.0))?;
    assert_eq!(service.config.target_bodyweight, Some(77.0));
    assert_eq!(service.get_target_bodyweight(), Some(77.0));

    // Clear the target
    service.set_target_bodyweight(None)?;
    assert!(service.config.target_bodyweight.is_none());
    assert!(service.get_target_bodyweight().is_none());

    // Test invalid input
    let result_neg = service.set_target_bodyweight(Some(-10.0));
    assert!(result_neg.is_err());
    match result_neg.unwrap_err() {
        ConfigError::InvalidBodyweightInput(_) => (),
        _ => panic!("Expected InvalidBodyweightInput error"),
    }

    Ok(())
}

#[test]
fn get_all_dates_exercised() -> Result<()> {
    let mut service = create_test_service()?;
    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;
    service.add_workout(
        "Bench Press",
        today,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    service.add_workout(
        "Bench Press",
        today,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    service.add_workout(
        "Bench Press",
        yesterday,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    let dates = service.get_all_dates_with_exercise()?;
    assert_eq!(dates.len(), 2);
    Ok(())
}

#[test]
fn test_graph_data_fetching() -> Result<()> {
    let mut service = create_test_service()?;
    let day1 = NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
    let day2 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(); // Add multiple entries day 2
    let day3 = NaiveDate::from_ymd_opt(2023, 10, 28).unwrap();

    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;
    service.create_exercise("Running", ExerciseType::Cardio, Some("legs"))?;

    // Add Bench Press data
    service.add_workout(
        "Bench Press",
        day1,
        Some(3),
        Some(10),
        Some(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // E1RM ~133.3, Vol=3000, Reps=30
    service.add_workout(
        "Bench Press",
        day2,
        Some(4),
        Some(8),
        Some(105.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // E1RM ~133.0, Vol=3360, Reps=32
    service.add_workout(
        "Bench Press",
        day2,
        Some(1),
        Some(6),
        Some(110.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // E1RM ~132.0, Vol=660, Reps=6 -- Max E1RM for day2 is 133.0
    service.add_workout(
        "Bench Press",
        day3,
        Some(2),
        Some(5),
        Some(110.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )?; // E1RM ~128.3, Vol=1100, Reps=10

    // Add Running data
    service.add_workout(
        "Running",
        day2,
        None,
        None,
        None,
        Some(30),
        Some(5.0),
        None,
        None,
        None,
        None,
    )?;
    service.add_workout(
        "Running",
        day2,
        None,
        None,
        None,
        Some(10),
        Some(2.0),
        None,
        None,
        None,
        None,
    )?; // Shorter run same day
    service.add_workout(
        "Running",
        day3,
        None,
        None,
        None,
        Some(35),
        Some(5.5),
        None,
        None,
        None,
        None,
    )?;

    // Test E1RM
    let e1rm_data = service.get_data_for_graph("Bench Press", GraphType::Estimated1RM)?;
    // Expected: One point per day with the MAX E1RM for that day
    assert_eq!(e1rm_data.len(), 3);
    assert_eq!(e1rm_data[0].0, 0.0); // Day 1 relative
    assert!((e1rm_data[0].1 - 133.33).abs() < 0.1);
    assert_eq!(e1rm_data[1].0, 1.0); // Day 2 relative - Max E1RM was 133.0
    assert!((e1rm_data[1].1 - 133.0).abs() < 0.1);
    assert_eq!(e1rm_data[2].0, 2.0); // Day 3 relative
    assert!((e1rm_data[2].1 - 128.33).abs() < 0.1);

    // Test Max Weight (should be actual weight used)
    let weight_data = service.get_data_for_graph("Bench Press", GraphType::MaxWeight)?;
    assert_eq!(weight_data.len(), 3);
    assert_eq!(weight_data[0].0, 0.0); // Day 1
    assert_eq!(weight_data[0].1, 100.0);
    assert_eq!(weight_data[1].0, 1.0); // Day 2 - Max weight was 110.0
    assert_eq!(weight_data[1].1, 110.0);
    assert_eq!(weight_data[2].0, 2.0); // Day 3
    assert_eq!(weight_data[2].1, 110.0);

    // Test Max Reps
    let reps_data = service.get_data_for_graph("Bench Press", GraphType::MaxReps)?;
    // Expected: One point per day with MAX reps from any set that day
    assert_eq!(reps_data.len(), 3);
    assert_eq!(reps_data[0].0, 0.0); // Day 1
    assert_eq!(reps_data[0].1, 10.0);
    assert_eq!(reps_data[1].0, 1.0); // Day 2 - Max reps was 8
    assert_eq!(reps_data[1].1, 8.0);
    assert_eq!(reps_data[2].0, 2.0); // Day 3
    assert_eq!(reps_data[2].1, 5.0);

    // Test Workout Volume
    let volume_data = service.get_data_for_graph("Bench Press", GraphType::WorkoutVolume)?;
    // Expected: One point per day with SUM of volume for that day
    assert_eq!(volume_data.len(), 3);
    assert_eq!(volume_data[0].0, 0.0); // Day 1: 3000
    assert!((volume_data[0].1 - 3000.0).abs() < 0.1);
    assert_eq!(volume_data[1].0, 1.0); // Day 2: 3360 + 660 = 4020
    assert!((volume_data[1].1 - 4020.0).abs() < 0.1);
    assert_eq!(volume_data[2].0, 2.0); // Day 3: 1100
    assert!((volume_data[2].1 - 1100.0).abs() < 0.1);

    // Test Workout Reps (Total reps for the day: sets * reps)
    let workout_reps_data = service.get_data_for_graph("Bench Press", GraphType::WorkoutReps)?;
    // Expected: One point per day with SUM of total reps for that day
    assert_eq!(workout_reps_data.len(), 3);
    assert_eq!(workout_reps_data[0].0, 0.0); // Day 1: 3*10 = 30
    assert_eq!(workout_reps_data[0].1, 30.0);
    assert_eq!(workout_reps_data[1].0, 1.0); // Day 2: (4*8) + (1*6) = 32 + 6 = 38
    assert_eq!(workout_reps_data[1].1, 38.0);
    assert_eq!(workout_reps_data[2].0, 2.0); // Day 3: 2*5 = 10
    assert_eq!(workout_reps_data[2].1, 10.0);

    // Test Workout Duration (Running)
    let duration_data = service.get_data_for_graph("Running", GraphType::WorkoutDuration)?;
    // Expected: One point per day with MAX duration for that day
    assert_eq!(duration_data.len(), 2);
    assert_eq!(duration_data[0].0, 0.0); // Day 2 relative to first running workout
    assert_eq!(duration_data[0].1, 40.0);
    assert_eq!(duration_data[1].0, 1.0); // Day 3 relative (only one entry)
    assert_eq!(duration_data[1].1, 35.0);

    // Test Workout Distance (Running - Metric)
    let distance_data_metric = service.get_data_for_graph("Running", GraphType::WorkoutDistance)?;
    // Expected: One point per day with MAX distance for that day
    assert_eq!(distance_data_metric.len(), 2);
    assert_eq!(distance_data_metric[0].0, 0.0); // Day 2 relative
    assert_eq!(distance_data_metric[0].1, 7.0); // km
    assert_eq!(distance_data_metric[1].0, 1.0); // Day 3 relative
    assert_eq!(distance_data_metric[1].1, 5.5); // km

    // Test Workout Distance (Running - Imperial)
    service.config.units = Units::Imperial;
    let distance_data_imperial =
        service.get_data_for_graph("Running", GraphType::WorkoutDistance)?;
    // Expected: One point per day with MAX distance for that day (converted to miles)
    assert_eq!(distance_data_imperial.len(), 2);
    assert_eq!(distance_data_imperial[0].0, 0.0); // Day 2 relative
    assert!((distance_data_imperial[0].1 - (7.0 * 0.621371)).abs() < 0.01); // miles
    assert_eq!(distance_data_imperial[1].0, 1.0); // Day 3 relative
    assert!((distance_data_imperial[1].1 - (5.5 * 0.621371)).abs() < 0.01); // miles

    // Test for exercise with no data
    service.create_exercise("Untouched", ExerciseType::Resistance, None)?;
    let no_data = service.get_data_for_graph("Untouched", GraphType::MaxWeight)?;
    assert!(no_data.is_empty());

    Ok(())
}

#[test]
fn test_list_all_muscles() -> Result<()> {
    let service = create_test_service()?;

    // No exercises yet
    assert!(service.list_all_muscles()?.is_empty());

    // Add exercises with various muscle strings
    service.create_exercise(
        "Bench Press",
        ExerciseType::Resistance,
        Some("Chest, Triceps"),
    )?;
    service.create_exercise(
        "Squat",
        ExerciseType::Resistance,
        Some("Legs, Glutes, Core"),
    )?;
    service.create_exercise("Pull-ups", ExerciseType::BodyWeight, Some("back, Biceps "))?; // Extra space
    service.create_exercise("Rows", ExerciseType::Resistance, Some("Back, Rear Delts"))?;
    service.create_exercise("Running", ExerciseType::Cardio, Some("Legs"))?; // Duplicate 'legs'
    service.create_exercise("Crunches", ExerciseType::BodyWeight, Some("core"))?; // Duplicate 'core', case difference
    service.create_exercise("Empty Muscle", ExerciseType::Resistance, Some(""))?; // Empty string
    service.create_exercise("Null Muscle", ExerciseType::Resistance, None)?; // Null value
    service.create_exercise("Just Comma", ExerciseType::Resistance, Some(","))?; // Just a comma
    service.create_exercise(
        "Leading Comma",
        ExerciseType::Resistance,
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
