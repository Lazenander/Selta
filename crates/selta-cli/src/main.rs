//! selta: thin client over the seltad HTTP API. The ✓/✗ rendering here is a
//! convenience of this one client, not part of the output contract (docs/04).

use std::io::Read;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use selta_core::{CheckResult, Children, NodeResult, Report, Verdict};

#[derive(Parser)]
#[command(name = "selta", about = "psql for Selta: pools, schemas, verification")]
struct Cli {
    /// Server base URL
    #[arg(long, global = true, default_value = "http://127.0.0.1:7466")]
    server: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage schema pools
    Pool {
        #[command(subcommand)]
        command: PoolCommand,
    },
    /// Manage schemas within a pool
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Verify a value against a schema
    Verify {
        pool: String,
        /// Schema reference: name or name@version
        schema: String,
        /// File with the value; '-' reads stdin
        file: String,
        /// Treat the input as raw model text (goes through intake)
        #[arg(long)]
        text: bool,
        /// Env entries KEY=VALUE; VALUE parsed as JSON when possible
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env: Vec<String>,
        #[arg(long)]
        wait_ms: Option<u64>,
        /// Print the raw JSON report instead of the rendered view
        #[arg(long)]
        json: bool,
    },
    /// Fetch a job's report; --watch polls until it settles
    Job {
        pool: String,
        id: String,
        #[arg(long)]
        watch: bool,
        #[arg(long)]
        json: bool,
    },
    /// List extensions: server manifests, or one pool's view with settings
    Extensions {
        #[arg(long)]
        pool: Option<String>,
    },
    /// Set pool-scope settings for an extension (inline JSON or a file path)
    Settings {
        pool: String,
        ext: String,
        /// JSON object, or a path to a file containing one; '-' reads stdin
        value: String,
    },
    /// Per-extension counters for a pool (calls, latency, usage, agreement)
    Stats { pool: String },
}

#[derive(Subcommand)]
enum PoolCommand {
    Create {
        name: String,
        /// Host extensions to enable (repeatable)
        #[arg(long = "extension")]
        extensions: Vec<String>,
        /// Command templates to enable (repeatable)
        #[arg(long = "cmd")]
        cmd: Vec<String>,
    },
    List,
    Show {
        name: String,
    },
}

#[derive(Subcommand)]
enum SchemaCommand {
    /// Register a schema file → a new immutable version
    Put {
        pool: String,
        name: String,
        file: String,
    },
    /// Fetch a schema: name or name@version
    Get { pool: String, reference: String },
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let server = cli.server.trim_end_matches('/').to_string();
    match cli.command {
        Command::Pool { command } => pool(&server, command),
        Command::Schema { command } => schema(&server, command),
        Command::Verify {
            pool,
            schema,
            file,
            text,
            env,
            wait_ms,
            json,
        } => verify(&server, &pool, &schema, &file, text, &env, wait_ms, json),
        Command::Job {
            pool,
            id,
            watch,
            json,
        } => job(&server, &pool, &id, watch, json),
        Command::Extensions { pool } => extensions(&server, pool.as_deref()),
        Command::Settings { pool, ext, value } => settings(&server, &pool, &ext, &value),
        Command::Stats { pool } => stats(&server, &pool),
    }
}

fn extensions(server: &str, pool: Option<&str>) -> Result<()> {
    let path = match pool {
        Some(pool) => format!("/pools/{pool}/extensions"),
        None => "/extensions".to_string(),
    };
    let (status, body) = get(server, &path)?;
    expect_ok(status, &body)?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn settings(server: &str, pool: &str, ext: &str, value: &str) -> Result<()> {
    let content = match serde_json::from_str::<Value>(value) {
        Ok(parsed) => parsed,
        Err(_) => serde_json::from_str(&read_input(value)?)
            .context("settings must be a JSON object (inline or in the file)")?,
    };
    let (status, body) = put(server, &format!("/pools/{pool}/settings/{ext}"), content)?;
    expect_ok(status, &body)?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn stats(server: &str, pool: &str) -> Result<()> {
    let (status, body) = get(server, &format!("/pools/{pool}/stats"))?;
    expect_ok(status, &body)?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn pool(server: &str, command: PoolCommand) -> Result<()> {
    match command {
        PoolCommand::Create {
            name,
            extensions,
            cmd,
        } => {
            let (status, body) = post(
                server,
                "/pools",
                json!({ "name": name, "extensions": extensions, "cmd": cmd }),
            )?;
            expect_ok(status, &body)?;
            println!("created pool '{name}'");
        }
        PoolCommand::List => {
            let (status, body) = get(server, "/pools")?;
            expect_ok(status, &body)?;
            for name in body["pools"].as_array().unwrap_or(&Vec::new()) {
                println!("{}", name.as_str().unwrap_or_default());
            }
        }
        PoolCommand::Show { name } => {
            let (status, body) = get(server, &format!("/pools/{name}"))?;
            expect_ok(status, &body)?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
    }
    Ok(())
}

fn schema(server: &str, command: SchemaCommand) -> Result<()> {
    match command {
        SchemaCommand::Put { pool, name, file } => {
            let content = read_input(&file)?;
            let schema: Value =
                serde_json::from_str(&content).context("schema file is not valid JSON")?;
            let (status, body) = put(server, &format!("/pools/{pool}/schemas/{name}"), schema)?;
            expect_ok(status, &body)?;
            println!("registered {}@{}", name, body["version"]);
        }
        SchemaCommand::Get { pool, reference } => {
            let (status, body) = get(server, &format!("/pools/{pool}/schemas/{reference}"))?;
            expect_ok(status, &body)?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn verify(
    server: &str,
    pool: &str,
    schema: &str,
    file: &str,
    text: bool,
    env_entries: &[String],
    wait_ms: Option<u64>,
    raw_json: bool,
) -> Result<()> {
    let content = read_input(file)?;
    let mut env = serde_json::Map::new();
    for entry in env_entries {
        let (key, value) = entry
            .split_once('=')
            .with_context(|| format!("bad --env entry '{entry}', expected KEY=VALUE"))?;
        let parsed = serde_json::from_str(value).unwrap_or(Value::String(value.to_string()));
        env.insert(key.to_string(), parsed);
    }
    let mut request = json!({ "schema": schema, "env": Value::Object(env) });
    if text {
        request["text"] = Value::String(content);
    } else {
        request["value"] = serde_json::from_str(&content)
            .context("input is not valid JSON — pass --text for raw model output")?;
    }
    if let Some(wait) = wait_ms {
        request["options"] = json!({ "wait_ms": wait });
    }
    let (status, body) = post(server, &format!("/pools/{pool}/verify"), request)?;
    if status == 202 {
        println!("job {}", body["job"].as_str().unwrap_or_default());
        return Ok(());
    }
    expect_ok(status, &body)?;
    output_report(body, raw_json)
}

fn job(server: &str, pool: &str, id: &str, watch: bool, raw_json: bool) -> Result<()> {
    loop {
        let (status, body) = get(server, &format!("/pools/{pool}/jobs/{id}"))?;
        if status == 202 {
            if body["status"] == "canceled" {
                println!("job {id}: canceled");
                return Ok(());
            }
            if watch {
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            println!("job {id}: running");
            return Ok(());
        }
        expect_ok(status, &body)?;
        return output_report(body, raw_json);
    }
}

fn output_report(body: Value, raw_json: bool) -> Result<()> {
    if raw_json {
        println!("{}", serde_json::to_string_pretty(&body)?);
        return Ok(());
    }
    let report: Report = serde_json::from_value(body).context("unexpected report shape")?;
    render(&report);
    Ok(())
}

fn render(report: &Report) {
    if let Some(schema) = &report.schema {
        println!("schema:  {schema}");
    }
    println!("verdict: {}", verdict_word(report.verdict));
    println!();
    let mut rows = Vec::new();
    collect(&report.root, &mut rows);
    for (path, check) in &rows {
        if check.source == "structure" && check.verdict == Verdict::Pass {
            continue;
        }
        let symbol = if check.skipped {
            "–"
        } else {
            match check.verdict {
                Verdict::Pass => "✓",
                Verdict::Fail => "✗",
                Verdict::Inconclusive => "?",
            }
        };
        let detail = if let Some(delta) = check.deltas.first() {
            first_line(&delta.message)
        } else if let Some(error) = &check.error {
            first_line(error)
        } else if check.skipped {
            "skipped: depth exhausted".to_string()
        } else {
            String::new()
        };
        let votes = check
            .votes
            .map(|t| format!("  [{}/{} votes pass]", t.pass, t.pass + t.fail))
            .unwrap_or_default();
        println!(
            "{symbol}  {:<20} {:<12} {detail}{votes}",
            path, check.source
        );
    }
    for notice in &report.notices {
        println!(
            "·  {:<20} {:<12} {}",
            notice.path, notice.source, notice.message
        );
    }
    let shown: std::collections::HashSet<(&str, &str)> = rows
        .iter()
        .filter(|(_, check)| check.error.is_some())
        .map(|(path, check)| (path.as_str(), check.source.as_str()))
        .collect();
    for error in &report.errors {
        if shown.contains(&(error.path.as_str(), error.source.as_str())) {
            continue;
        }
        println!(
            "?  {:<20} {:<12} {}",
            error.path,
            error.source,
            first_line(&error.error)
        );
    }
    println!();
    println!(
        "executions: {}   tokens: {}/{}   cost: ${:.4}   elapsed: {} ms",
        report.usage.samples,
        report.usage.input_tokens,
        report.usage.output_tokens,
        report.usage.cost_usd,
        report.timing.elapsed_ms
    );
}

fn collect(node: &NodeResult, out: &mut Vec<(String, CheckResult)>) {
    for check in &node.checks {
        out.push((node.path.clone(), check.clone()));
    }
    match &node.children {
        Some(Children::Fields(fields)) => {
            for child in fields.values() {
                collect(child, out);
            }
        }
        Some(Children::Items(items)) => {
            for child in items {
                collect(child, out);
            }
        }
        None => {}
    }
}

fn verdict_word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Inconclusive => "inconclusive",
    }
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().to_string()
}

fn read_input(file: &str) -> Result<String> {
    if file == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        std::fs::read_to_string(file).with_context(|| format!("reading {file}"))
    }
}

// --- tiny HTTP helpers over ureq ---

fn get(server: &str, path: &str) -> Result<(u16, Value)> {
    handle(ureq::get(&format!("{server}{path}")).call())
}

fn post(server: &str, path: &str, body: Value) -> Result<(u16, Value)> {
    handle(ureq::post(&format!("{server}{path}")).send_json(body))
}

fn put(server: &str, path: &str, body: Value) -> Result<(u16, Value)> {
    handle(ureq::put(&format!("{server}{path}")).send_json(body))
}

fn handle(result: Result<ureq::Response, ureq::Error>) -> Result<(u16, Value)> {
    match result {
        Ok(response) => {
            let status = response.status();
            let body = response.into_json().unwrap_or(json!({}));
            Ok((status, body))
        }
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_json().unwrap_or(json!({}));
            Ok((status, body))
        }
        Err(error) => bail!("cannot reach seltad: {error}"),
    }
}

fn expect_ok(status: u16, body: &Value) -> Result<()> {
    if (200..300).contains(&status) {
        return Ok(());
    }
    let message = body["error"].as_str().unwrap_or("unknown error");
    bail!("server returned {status}: {message}")
}
