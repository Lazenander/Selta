mod artifact;
mod cli;
mod corpus;
mod digest;
mod finite;
mod inventory;
mod json;
mod path;
mod stable;

fn main() -> anyhow::Result<()> {
    cli::run()
}
