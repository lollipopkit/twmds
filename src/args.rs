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
}
