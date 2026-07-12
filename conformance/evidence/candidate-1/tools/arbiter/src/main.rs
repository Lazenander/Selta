mod artifact;
mod cli;
mod digest;
mod finite;
mod json;
mod path;
mod stable;

fn main() -> anyhow::Result<()> {
    cli::run()
}
