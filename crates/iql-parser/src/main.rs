use issuecraft_ql::parse_query;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args
        .iter()
        .any(|arg| arg == "-h" || arg == "--help" || arg == "help")
    {
        print_help();
        std::process::exit(0);
    }

    if args.len() > 1 {
        let query = args[1..].join(" ");
        let parsed = parse_query(&query);

        match parsed {
            Ok(statement) => {
                println!("✓ Parse successful!");
                println!();
                println!("Query: {}", query);
                println!();
                println!("Parsed AST:");
                println!("{statement:#?}");
            }
            Err(error) => {
                eprintln!("✗ Parse error!");
                eprintln!();
                eprintln!("Query: {}", query);
                eprintln!();
                eprintln!("Error: {}", error);
                eprintln!();
                eprintln!("If you need help, run `issuecraft-ql help`");

                std::process::exit(1);
            }
        }
    } else {
        println!("Usage: issuecraft-ql [...query]");
    }
}

fn print_help() {
    println!();
    println!("IssueCraft Query Language (IQL) Help");
    println!("===================");
    println!();
    println!("CREATE Statements:");
    println!("  CREATE USER <username> [WITH EMAIL <email> NAME '<name>']");
    println!(
        "  CREATE PROJECT <project-id> [WITH NAME '<name>' DESCRIPTION '<desc>' OWNER <username>]"
    );
    println!(
        "  CREATE ISSUE IN <project> WITH TITLE '<title>' [DESCRIPTION '<desc>'] [PRIORITY <level>] [ASSIGNEE <user>]"
    );
    println!("  CREATE COMMENT ON ISSUE <id> WITH '<content>'");
    println!();
    println!("SELECT Statements:");
    println!("  SELECT * FROM <entity>");
    println!("  SELECT <col1>, <col2>, ... FROM <entity>");
    println!("  SELECT ... WHERE <condition>");
    println!("  SELECT ... ORDER BY <field> [ASC|DESC]");
    println!("  SELECT ... LIMIT <n> [OFFSET <n>]");
    println!();
    println!("UPDATE Statements:");
    println!("  UPDATE <entity-type> <id> SET <field> = <value>[, ...]");
    println!();
    println!("DELETE Statements:");
    println!("  DELETE <entity-type> <id>");
    println!();
    println!("Other Statements:");
    println!("  ASSIGN ISSUE <id> TO <username>");
    println!("  CLOSE ISSUE <id> [WITH '<reason>']");
    println!("  COMMENT ON ISSUE <id> WITH '<content>'");
    println!();
    println!("Entity Types: USER, PROJECT, ISSUE, USERS, PROJECTS, ISSUES, COMMENTS");
    println!("Priority Levels: critical, high, medium, low");
    println!("Issue ID format: <project#number> (e.g., 'PROJ#123')");
    println!();
}
