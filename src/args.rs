use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    version = env!("CARGO_PKG_VERSION"), 
    author = env!("CARGO_PKG_AUTHORS"), 
    about = env!("CARGO_PKG_DESCRIPTION"), 
    name = env!("CARGO_PKG_NAME")
)]
pub struct Args {
    #[clap(short, long, help = "Do not login")]
    pub no_login: bool,
    #[clap(short, long, help = "Sleep seconds", default_value_t = 15)]
    pub sleep: u64,
    #[arg(short = 'd', long, help = "Skip dirs")]
    pub skip_dirs: Vec<String>,
}
