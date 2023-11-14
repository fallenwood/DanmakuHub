pub fn setup_db(dbpath: &str) {
  let connection = sqlite::open(dbpath).unwrap();

  let query = "create table if not exists Hashes(
    Id INTEGER PRIMARY KEY AUTOINCREMENT,
    Filename VARCHAR(1024) NOT NULL UNIQUE,
    Hash VARCHAR(1024) NOT NULL);
    create table if not exists Visits(
      Id INTEGER PRIMARY KEY AUTOINCREMENT,
      Filename VARCHAR(1024) NOT NULL UNIQUE,
      Visits INTEGER NOT NULL DEFAULT 1
    );
  ";

  connection.execute(query).unwrap();

  tracing::info!("db setup done");
}

pub fn insert_db(dbpath: &str, filename: &str, hash: &str) -> Option<sqlite::Error> {
  let connection = sqlite::open(dbpath).unwrap();

  let query = "insert into Hashes (Filename, Hash)
  values (?, ?)
  on conflict (Filename) do
  update set Hash = ?;";
  let mut statement = connection.prepare(query).unwrap();
  statement.bind((1, filename)).unwrap();
  statement.bind((2, hash)).unwrap();
  statement.bind((3, hash)).unwrap();

  statement.next().err()
}

pub fn update_visits(dbpath: &str, filename: &str) -> Result<i64, sqlite::Error> {
  let connection = sqlite::open(dbpath).unwrap();

  {
    let query = "insert into Visits (Filename)
    values (?)
    on conflict (Filename) do
    update set Visits = Visits+1;";
    let mut statement = connection.prepare(query).unwrap();
    statement.bind((1, filename)).unwrap();

    let next = statement.next();

    if next.is_err() {
      return Err(next.err().unwrap());
    }
  }

  {
    let query = "select visits from Visits where Filename=? limit 1";
    let mut statement = connection.prepare(query).unwrap();
    statement.bind((1, filename)).unwrap();

    let next = statement.next();
    if let Ok(state) = next {
      if state == sqlite::State::Row {
        return statement.read::<i64, usize>(0);
      }
    }

    return Err(next.err().unwrap());
  }
}

pub fn query_db(dbpath: &str, filename: &str) -> Option<String> {
  let connection = sqlite::open(dbpath).unwrap();

  let query = "select hash from Hashes where Filename=? limit 1";
  let mut statement = connection.prepare(query).unwrap();
  statement.bind((1, filename)).unwrap();

  if let Ok(state) = statement.next() {
    if state == sqlite::State::Row {
      if let Ok(hash) = statement.read::<String, usize>(0) {
        return Some(hash);
      }
    }
  }

  None
}
