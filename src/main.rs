use payengine::{accounts::ClientsDatabase, parser::Row};
use std::io::{BufRead, BufReader};
use tracing::trace;

fn main() {
    // set e.g. RUST_LOG=trace to debug
    tracing_subscriber::fmt::init();

    let filename = std::env::args()
        .nth(1)
        .expect("expected one argument - filename");
    let file = std::fs::File::open(&filename).expect("error opening file");
    let mut file = BufReader::new(file);

    let mut buf = Vec::<u8>::new();
    let mut db = ClientsDatabase::default();

    // skip header. Ignore parsing it either, assume it has fixed format.
    let _ = file
        .read_until(b'\n', &mut buf)
        .expect("error reading CSV header");

    // Parse and process all the rows.
    loop {
        buf.clear();
        let sz = file.read_until(b'\n', &mut buf).expect("error reading");
        if sz == 0 {
            break;
        }
        let line = &buf[..sz];
        let row = match Row::parse(line) {
            Ok(row) => row,
            Err(e) => {
                trace!("error parsing line {:?}: {e}", std::str::from_utf8(line));
                continue;
            }
        };
        if let Err(e) = db.process_transaction(row.client_id, row.transaction) {
            trace!(?row, "error processing transaction: {e}")
        }
    }

    // Print all client accounts
    println!("client, available, held, total, locked");
    for (client_id, account) in db.iter() {
        let available = account.available_for_withdrawal();
        let held = account.held();
        let total = account.total();
        let locked = account.is_frozen();
        println!("{client_id},{available},{held},{total},{locked}")
    }
}
