use sqlx::{MySql, Transaction};

pub type DatabaseTransaction<'a> = Transaction<'a, MySql>;
