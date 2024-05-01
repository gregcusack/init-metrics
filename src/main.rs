use {
    clap::{App, Arg},
    reqwest::{
        blocking::Client,
        header,
    },
    rpassword::prompt_password_stdout,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Initialize Metrics")
        .version("1.0")
        .about("Sets up metrics")
        .arg(Arg::with_name("username")
            .long("username")
            .short('u')
            .required(true)
            .help("InfluxDB user with access to create a new database")
            .takes_value(true))
        .arg(Arg::with_name("delete")
            .short('d')
            .long("delete")
            .help("Delete the database instead of creating it"))
        .arg(Arg::with_name("metrics_db")
            .short('c')
            .long("metrics-db")
            .required(true)
            .help("Manually specify a database to create")
            .takes_value(true))
        .get_matches();

    let host = "https://internal-metrics.solana.com:8086";
    let delete = matches.is_present("delete");
    let username = matches.value_of("username").expect("username not specified");
    let db_name = matches.value_of("metrics_db").unwrap();
    let password = prompt_password_stdout("InfluxDB password: ").unwrap();

    let url = format!("{host}/query?u={username}&p={password}");
    let query = format!("DROP DATABASE \"{}\"", db_name);
    let body = format!("q={query}");

    let mut headers = header::HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/x-www-form-urlencoded".parse().unwrap());

    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .https_only(true)
        .build()?;

    let response = client.post(&url)
        .headers(headers.clone())
        .body(body.clone())
        .send()?;

    if response.status().is_client_error() || response.status().is_server_error() {
        println!("Query '{body}' received an error response: {}", response.status());
    } else {
        println!("Query '{body}' was successful: {}", response.status());
    }

    if !delete {
        let queries = vec![
            format!("Create DATABASE \"{}\"", db_name),
            format!("ALTER RETENTION POLICY autogen ON \"{}\" DURATION 7d", db_name),
            format!("GRANT READ ON \"{}\" TO \"ro\"", db_name),
            format!("GRANT WRITE ON \"{}\" TO \"scratch_writer\"", db_name),
        ];

        for query in queries {
            let body = format!("q={query}");
            client.post(&url)
                .headers(headers.clone())
                .body(body.clone())
                .send()?;

            if response.status().is_client_error() || response.status().is_server_error() {
                println!("Query '{body}' received an error response: {}", response.status());
            } else {
                println!("Query '{body}' was successful: {}", response.status());
            }
        }
        let metrics_config = format!("host={},db={},u=scratch_writer,p=topsecret", host, db_name);
        println!("SOLANA_METRICS_CONFIG={metrics_config}");
    }
    
    Ok(())
}
