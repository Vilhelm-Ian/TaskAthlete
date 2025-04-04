use anyhow::Result;
use chrono::{Utc, Duration, NaiveDate}; // Import NaiveDate
use workout_tracker_lib::{
    AppService, Config, ConfigError, DbError, ExerciseType, Units, 
    WorkoutFilters,
    VolumeFilters, PBInfo, PBType, PbMetricScope,
};
use std::thread; // For adding delays in PB tests
use std::time::Duration as StdDuration; // For delays


// Helper function to create a test service with in-memory database
fn create_test_service() -> Result<AppService> {
    // Create an in-memory database for testing
    let conn = rusqlite::Connection::open_in_memory()?;
    workout_tracker_lib::db::init_db(&conn)?;

    // Create a default config for testing
    let config = Config {
        bodyweight: Some(70.0), // Set a default bodyweight for tests
        units: Units::Metric,
        prompt_for_bodyweight: true,
        ..Default::default()
    };

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
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Try creating with same name (case-insensitive)
    let result = service.create_exercise("bench press", ExerciseType::Cardio, None);
    assert!(result.is_err());
    // Check for the specific error type/message if desired
    assert!(result.unwrap_err().to_string().contains("Exercise name must be unique"));

    // Try creating with different name
    let result = service.create_exercise("Squat", ExerciseType::Resistance, Some("legs"));
    assert!(result.is_ok());

    Ok(())
}


#[test]
fn test_exercise_aliases() -> Result<()> {
    let mut service = create_test_service()?;

    let ex_id = service.create_exercise("Barbell Bench Press", ExerciseType::Resistance, Some("chest"))?;
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
     println!("{:?}",result);
     assert!(result.unwrap_err().to_string().contains("Alias already exists"));

     // 5. Try creating alias conflicting with name/id
     let result = service.create_alias("Barbell Bench Press", "Squat"); // Alias conflicts with name
     assert!(result.is_err());
     assert!(result.unwrap_err().to_string().contains("conflicts with an existing exercise name"));

     let result = service.create_alias(&ex_id.to_string(), "Squat"); // Alias conflicts with ID
     assert!(result.is_err());
     assert!(result.unwrap_err().to_string().contains("conflicts with an existing exercise ID"));


    // 6. Use Alias in Add Workout
    let today = Utc::now().date_naive();
    let (workout_id, _) = service.add_workout("bp", today, Some(3), Some(5), Some(100.0), None, None, None, None, None)?;
    let workouts = service.list_workouts(WorkoutFilters{ exercise_name: Some("bp"), ..Default::default() })?;
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

// TODO
// #[test]
// fn test_edit_exercise_with_alias_and_name_change() -> Result<()> {
//     // Need mutable connection for the transaction in db::update_exercise
//     // AppService doesn't hold mut conn, so create one separately
//     let mut service = create_test_service()?; // Setup schema and initial data via service
//     let mut conn = create_mutable_conn_to_test_db()?; // Get a mutable connection

//     service.create_exercise("Old Name", ExerciseType::Resistance, Some("muscle1"))?;
//     service.create_alias("on", "Old Name")?;

//     // Add a workout using alias
//     let today = Utc::now().date_naive();
//     service.add_workout("on", today, Some(1), Some(1), Some(1.0), None, None, None, None, None)?;

//     // Edit using alias, change name
//     // Use the separate mutable connection for the update operation
//     let canonical_name = service.resolve_identifier_to_canonical_name("on")?.unwrap();
//     workout_tracker_lib::db::update_exercise(
//         &mut conn, // Pass the mutable connection
//         &canonical_name,
//         Some("New Name"),
//         None,
//         Some(Some("muscle1,muscle2")),
//     )?;

//     // Verify changes using service (which uses its own immutable connection)
//     // Check old alias points to new name (DB function handles this)
//     let aliases = service.list_aliases()?;
//     assert_eq!(aliases.get("on").unwrap(), "New Name");

//     // Check definition update
//     let new_def = service.resolve_exercise_identifier("on")?.unwrap();
//     assert_eq!(new_def.name, "New Name");
//     assert_eq!(new_def.muscles, Some("muscle1,muscle2".to_string()));

//     // Check workout entry was updated
//     let workouts = service.list_workouts(WorkoutFilters { exercise_name: Some("on"), ..Default::default() })?;
//     assert_eq!(workouts[0].exercise_name, "New Name");

//     Ok(())
// }

#[test]
fn test_delete_exercise_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("To Delete", ExerciseType::Cardio, None)?;
    service.create_alias("td", "To Delete")?;

    // Delete exercise using alias
    let result = service.delete_exercise("td")?;
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

    service.add_workout("Rowing", yesterday, None, None, None, Some(30), None, None, None, None)?;
    service.add_workout("Rowing", two_days_ago, None, None, None, Some(25), None, None, None, None)?;

    // List for yesterday
    let workouts_yesterday = service.list_workouts(WorkoutFilters{ date: Some(yesterday), ..Default::default() })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].duration_minutes, Some(30));
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);

    // List for two days ago
    let workouts_two_days_ago = service.list_workouts(WorkoutFilters{ date: Some(two_days_ago), ..Default::default() })?;
    assert_eq!(workouts_two_days_ago.len(), 1);
    assert_eq!(workouts_two_days_ago[0].duration_minutes, Some(25));
    assert_eq!(workouts_two_days_ago[0].timestamp.date_naive(), two_days_ago);


    Ok(())
}

#[test]
fn test_edit_workout_date() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Push-ups", ExerciseType::BodyWeight, None)?;
    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);

    let (workout_id, _) = service.add_workout("Push-ups", today, Some(3), Some(15), None, None, None, None, None, Some(70.0))?;

    // Edit the date
    service.edit_workout(workout_id, None, None, None, None, None, None, Some(yesterday))?;

    // Verify date change by listing
    let workouts_today = service.list_workouts(WorkoutFilters{ date: Some(today), ..Default::default() })?;
    assert!(workouts_today.is_empty());

    let workouts_yesterday = service.list_workouts(WorkoutFilters{ date: Some(yesterday), ..Default::default() })?;
    assert_eq!(workouts_yesterday.len(), 1);
    assert_eq!(workouts_yesterday[0].id, workout_id);
    assert_eq!(workouts_yesterday[0].timestamp.date_naive(), yesterday);


    Ok(())
}


#[test]
fn test_pb_detection_with_scope() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Deadlift", ExerciseType::Resistance, Some("back,legs"))?;
    let today = Utc::now().date_naive();

    // --- Scope: All ---
    service.set_pb_scope(PbMetricScope::All)?;

    // Workout 1: Baseline
    let (_, pb1) = service.add_workout("Deadlift", today, Some(1), Some(5), Some(100.0), None, None, None, None, None)?;
    assert!(pb1.is_none(), "Scope All: First workout shouldn't be a PB");
    thread::sleep(StdDuration::from_millis(10));

    // Workout 2: Weight PB
    let (_, pb2) = service.add_workout("Deadlift", today, Some(1), Some(3), Some(110.0), None, None, None, None, None)?;
    assert!(pb2.is_some(), "Scope All: Should detect weight PB");
    assert_eq!(pb2.as_ref().unwrap().pb_type, PBType::Weight);
    assert_eq!(pb2.as_ref().unwrap().new_weight, Some(110.0));
    thread::sleep(StdDuration::from_millis(10));

    // Workout 3: Reps PB
    let (_, pb3) = service.add_workout("Deadlift", today, Some(3), Some(6), Some(90.0), None, None, None, None, None)?;
    assert!(pb3.is_some(), "Scope All: Should detect reps PB");
    assert_eq!(pb3.as_ref().unwrap().pb_type, PBType::Reps);
    assert_eq!(pb3.as_ref().unwrap().new_reps, Some(6));
    thread::sleep(StdDuration::from_millis(10));

     // Workout 4: Both PB
    let (_, pb4) = service.add_workout("Deadlift", today, Some(1), Some(7), Some(120.0), None, None, None, None, None)?;
    assert!(pb4.is_some(), "Scope All: Should detect both PB");
    assert_eq!(pb4.as_ref().unwrap().pb_type, PBType::Both);
    assert_eq!(pb4.as_ref().unwrap().new_weight, Some(120.0));
    assert_eq!(pb4.as_ref().unwrap().new_reps, Some(7));
    thread::sleep(StdDuration::from_millis(10));

    // Workout 5: No PB
    let (_, pb5) = service.add_workout("Deadlift", today, Some(5), Some(5), Some(105.0), None, None, None, None, None)?;
    assert!(pb5.is_none(), "Scope All: Should not detect PB");
    thread::sleep(StdDuration::from_millis(10));

    // --- Scope: Weight Only ---
    service.set_pb_scope(PbMetricScope::Weight)?;

    // Workout 6: Weight PB (should be detected)
    let (_, pb6) = service.add_workout("Deadlift", today, Some(1), Some(4), Some(130.0), None, None, None, None, None)?;
    assert!(pb6.is_some(), "Scope Weight: Should detect weight PB");
    assert_eq!(pb6.as_ref().unwrap().pb_type, PBType::Weight);
    assert_eq!(pb6.as_ref().unwrap().new_weight, Some(130.0));
    thread::sleep(StdDuration::from_millis(10));

    // Workout 7: Reps PB (should NOT be detected as PB)
    let (_, pb7) = service.add_workout("Deadlift", today, Some(1), Some(8), Some(125.0), None, None, None, None, None)?;
    assert!(pb7.is_none(), "Scope Weight: Should NOT detect reps PB");
    thread::sleep(StdDuration::from_millis(10));

    // --- Scope: Reps Only ---
    service.set_pb_scope(PbMetricScope::Reps)?;

    // Workout 8: Reps PB (should be detected)
    let (_, pb8) = service.add_workout("Deadlift", today, Some(1), Some(9), Some(110.0), None, None, None, None, None)?;
    assert!(pb8.is_some(), "Scope Reps: Should detect reps PB");
    assert_eq!(pb8.as_ref().unwrap().pb_type, PBType::Reps);
    assert_eq!(pb8.as_ref().unwrap().new_reps, Some(9));
    thread::sleep(StdDuration::from_millis(10));

    // Workout 9: Weight PB (should NOT be detected as PB)
    let (_, pb9) = service.add_workout("Deadlift", today, Some(1), Some(5), Some(140.0), None, None, None, None, None)?;
    assert!(pb9.is_none(), "Scope Reps: Should NOT detect weight PB");

    Ok(())
}


#[test]
fn test_create_and_list_exercises() -> Result<()> {
    let service = create_test_service()?;

    // Create some exercises
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest,triceps"))?;
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
    service.set_pb_notification(true)?;

    // Check initial state
    assert_eq!(service.check_pb_notification_config()?, true);

    // Disable PB notifications
    service.set_pb_notification(false)?;
    assert_eq!(service.config.notify_on_pb, Some(false));
    assert_eq!(service.check_pb_notification_config()?, false);

     // Re-enable PB notifications
     service.set_pb_notification(true)?;
     assert_eq!(service.config.notify_on_pb, Some(true));
     assert_eq!(service.check_pb_notification_config()?, true);

    // Test case where config starts as None (simulate first run)
    service.config.notify_on_pb = None;
    let result = service.check_pb_notification_config();
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::PbNotificationNotSet => {}, // Correct error
        _ => panic!("Expected PbNotificationNotSet error"),
    }


    Ok(())
}

// Test list filtering with aliases
#[test]
fn test_list_filter_with_alias() -> Result<()> {
    let mut service = create_test_service()?;
    service.create_exercise("Overhead Press", ExerciseType::Resistance, Some("shoulders"))?;
    service.create_alias("ohp", "Overhead Press")?;
    let today = Utc::now().date_naive();

    service.add_workout("ohp", today, Some(5), Some(5), Some(50.0), None, None, None, None, None)?;

    // Filter list using alias
    let workouts = service.list_workouts(WorkoutFilters { exercise_name: Some("ohp"), ..Default::default() })?;
    assert_eq!(workouts.len(), 1);
    assert_eq!(workouts[0].exercise_name, "Overhead Press");

    // Filter list using canonical name
    let workouts2 = service.list_workouts(WorkoutFilters { exercise_name: Some("Overhead Press"), ..Default::default() })?;
    assert_eq!(workouts2.len(), 1);
    assert_eq!(workouts2[0].exercise_name, "Overhead Press");

    Ok(())
}


#[test]
fn test_add_and_list_workouts() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise first
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Add some workouts
    service.add_workout(
        "Bench Press",
        NaiveDate::from_ymd_opt(2015, 6, 2).unwrap(),
        Some(3),
        Some(10),
        Some(60.0),
        None,
        Some("First workout".to_string()),
        None,
        None,
        None,
    )?;

    service.add_workout(
        "Bench Press",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        Some(4),
        Some(8),
        Some(70.0),
        None,
        Some("Second workout".to_string()),
        None,
        None,
        None,
    )?;

    // List workouts
    let workouts = service.list_workouts(WorkoutFilters {
        exercise_name: Some("Bench Press"),
        ..Default::default()
    })?;

    assert_eq!(workouts.len(), 2);
    assert_eq!(workouts[0].sets, Some(4)); // Most recent first
    assert_eq!(workouts[1].sets, Some(3));

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
    assert_eq!(exercise.muscles, Some("chest,triceps,shoulders".to_string()));

    Ok(())
}

#[test]
fn test_delete_exercise() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Bench Press", ExerciseType::Resistance, Some("chest"))?;

    // Delete it
    let result = service.delete_exercise("Bench Press")?;
    assert_eq!(result, 1);

    // Verify it's gone
    let exercise = service.get_exercise_by_identifier_service("Bench Press")?;
    assert!(exercise.is_none());

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
    service.add_workout(
        "Running",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        None,
        None,
        None,
        Some(30),
        None,
        None,
        None,
        None,
    )?;

    // Add another workout
    service.add_workout(
        "Curl",
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
        None,
        None,
        None,
        Some(45),
        None,
        None,
        None,
        None,
    )?;

    // Filter by type
    let resistance_workout = service.list_workouts(WorkoutFilters {
        exercise_type: Some(ExerciseType::Resistance),
        ..Default::default()
    })?;
    assert_eq!(resistance_workout.len(), 1);
    assert_eq!(resistance_workout[0].duration_minutes, Some(45));


    Ok(())
}

#[test]
fn test_nth_last_day_workouts() -> Result<()> {
    let mut service = create_test_service()?;

    // Create an exercise
    service.create_exercise("Squats", ExerciseType::Resistance, Some("legs"))?;

    // Add workouts on different dates
    // First workout (older)
    service.add_workout(
        "Squats",
        NaiveDate::from_ymd_opt(2015, 6, 2).unwrap(),
        Some(3),
        Some(10),
        Some(100.0),
        None,
        Some("First workout".to_string()),
        None,
        None,
        None,
    )?;

    // Second workout (more recent)
    service.add_workout(
        "Squats",
        NaiveDate::from_ymd_opt(2015, 6, 7).unwrap(),
        Some(5),
        Some(5),
        Some(120.0),
        None,
        Some("Second workout".to_string()),
        None,
        None,
        None,
    )?;

    // Get workouts for the most recent day (n=1)
    let recent_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 1)?;
    assert_eq!(recent_workouts.len(), 1);
    assert_eq!(recent_workouts[0].sets, Some(5));

    // Get workouts for the previous day (n=2)
    let previous_workouts = service.list_workouts_for_exercise_on_nth_last_day("Squats", 2)?;
    assert_eq!(previous_workouts.len(), 1);
    assert_eq!(previous_workouts[0].sets, Some(3));

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

    Ok(())
}

#[test]
fn test_workout_not_found() -> Result<()> {
    let service = create_test_service()?;

    // Try to edit non-existent workout
    let result = service.edit_workout(999, None, None, None, None, None, None, None);
    println!("testing {:?}", result);
    assert!(result.is_err());

    // Try to delete non-existent workout
    let result = service.delete_workout(999);
    assert!(result.is_err());

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
    // Entry 1: Bench Press
    service.add_workout("Bench Press", day1, Some(3), Some(10), Some(100.0), None, None, None, None, None)?; // Vol = 3*10*100 = 3000
    // Entry 2: Bench Press (another set)
    service.add_workout("Bench Press", day1, Some(1), Some(8), Some(105.0), None, None, None, None, None)?; // Vol = 1*8*105 = 840
    // Entry 3: Pullups (Bodyweight 70kg + 10kg)
    service.add_workout("Pull-ups", day1, Some(4), Some(6), Some(10.0), None, None, None, None, Some(70.0))?; // Vol = 4*6*(70+10) = 1920
     // Entry 4: Running (Cardio - should have 0 volume)
    service.add_workout("Running", day1, None, None, None, Some(30), None, None, None, None)?; // Vol = 0

    // Day 2 Workouts
    service.add_workout("Squats", day2, Some(5), Some(5), Some(120.0), None, None, None, None, None)?; // Vol = 5*5*120 = 3000
    service.add_workout("Bench Press", day2, Some(4), Some(6), Some(100.0), None, None, None, None, None)?; // Vol = 4*6*100 = 2400

    // --- Test Volume Calculation ---

    // Total volume per day (no filters, default limit)
    let volume_all = service.calculate_daily_volume(VolumeFilters::default())?;
    assert_eq!(volume_all.len(), 2); // Day 1 and Day 2
    // Day 2 should be first (most recent)
    assert_eq!(volume_all[0].0, day2);
    assert!((volume_all[0].1 - (3000.0 + 2400.0)).abs() < 0.01); // Day 2 Total = 5400
    assert_eq!(volume_all[1].0, day1);
    assert!((volume_all[1].1 - (3000.0 + 840.0 + 1920.0 + 0.0)).abs() < 0.01); // Day 1 Total = 5760

    // Volume for Day 1 only
    let volume_day1 = service.calculate_daily_volume(VolumeFilters {
        start_date: Some(day1), end_date: Some(day1), ..Default::default()
    })?;
    assert_eq!(volume_day1.len(), 1);
    assert_eq!(volume_day1[0].0, day1);
    assert!((volume_day1[0].1 - 5760.0).abs() < 0.01);

    // Volume for "Bench Press" only
    let volume_bp = service.calculate_daily_volume(VolumeFilters {
        exercise_name: Some("Bench Press"), ..Default::default()
    })?;
    assert_eq!(volume_bp.len(), 2);
     // Day 2 BP
    assert_eq!(volume_bp[0].0, day2);
    assert!((volume_bp[0].1 - 2400.0).abs() < 0.01);
    // Day 1 BP
    assert_eq!(volume_bp[1].0, day1);
    assert!((volume_bp[1].1 - (3000.0 + 840.0)).abs() < 0.01); // Sum of both BP entries

    // Volume for Cardio (should be 0)
    let volume_cardio = service.calculate_daily_volume(VolumeFilters {
        exercise_type: Some(ExerciseType::Cardio), ..Default::default()
    })?;
    // Check if day1 exists and volume is 0, or if the vec is empty (if no other cardio days)
    assert!(volume_cardio.is_empty() || (volume_cardio[0].0 == day1 && volume_cardio[0].1 == 0.0));

    Ok(())
}
