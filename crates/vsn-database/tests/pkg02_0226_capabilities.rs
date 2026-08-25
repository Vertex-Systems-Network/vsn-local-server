use vsn_database::{remote_database_capabilities, validate_remote_database_capabilities};

#[test]
fn external_native_beta_matrix_is_exact_and_truthful() {
    let report = validate_remote_database_capabilities();
    assert!(report.valid, "{:?}", report.issues);
    let engines = report
        .engines
        .iter()
        .map(|engine| engine.engine.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        engines,
        ["postgresql", "mysql", "mariadb", "mongodb", "redis"]
    );
    assert!(report
        .engines
        .iter()
        .all(|engine| engine.plaintext_loopback && engine.verified_tls_remote));
    assert!(!report
        .engines
        .iter()
        .any(|engine| engine.engine == "sqlite"));
    let maria = report
        .engines
        .iter()
        .find(|engine| engine.engine == "mariadb")
        .unwrap();
    assert!(maria.query);
    assert!(!maria.write);
    let mongo = report
        .engines
        .iter()
        .find(|engine| engine.engine == "mongodb")
        .unwrap();
    assert!(mongo.browse && mongo.write && !mongo.query);
    let redis = report
        .engines
        .iter()
        .find(|engine| engine.engine == "redis")
        .unwrap();
    assert!(redis.write && !redis.query && !redis.browse);
    assert_eq!(remote_database_capabilities().len(), 5);
}
