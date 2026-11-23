use anyhow::Result;
use ascii_shadow::app;
use ascii_shadow::cli::Args;

fn main() -> Result<()> {
    let args = Args::parse_args()?;
    app::run(&args)
}
