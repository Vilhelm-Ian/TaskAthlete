use anyhow::Result;
use chrono::NaiveDate;
use workout_tracker_lib::{
    AppService, Config, ConfigError, DbError, ExerciseType, Units, 
    WorkoutFilters,
};

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
        NaiveDate::from_ymd_opt(2015, 6, 3).unwrap(),
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
