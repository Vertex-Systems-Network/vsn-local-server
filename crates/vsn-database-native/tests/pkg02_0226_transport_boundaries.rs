use vsn_database_native::{
    mysql_inspect, postgres_inspect, MySqlConnection, NativeDbError, PostgresConnection,
};

#[test]
fn postgres_plaintext_remote_hostname_prefix_fails_closed_before_connect() {
    let err = postgres_inspect(&PostgresConnection {
        connection_string:
            "host=localhost.evil.invalid port=5432 user=vsn dbname=vsn connect_timeout=1".into(),
    })
    .expect_err("remote hostname with localhost prefix must be rejected before connect");

    assert!(
        matches!(err, NativeDbError::Invalid(_)),
        "expected fail-closed validation error, got: {err:?}"
    );
}

#[test]
fn mysql_plaintext_remote_hostname_prefix_fails_closed_before_connect() {
    let err = mysql_inspect(&MySqlConnection {
        url: "mysql://vsn@localhost.evil.invalid:3306/vsn".into(),
    })
    .expect_err("remote hostname with localhost prefix must be rejected before connect");

    assert!(
        matches!(err, NativeDbError::Invalid(_)),
        "expected fail-closed validation error, got: {err:?}"
    );
}
