// CLI for AirGapSync

use clap::Parser;

#[derive(Parser)]
#[clap(name = "AirGapSync", version = "0.1.0", about = "Encrypted Removable-Media Sync CLI")]
pub struct Args {
    #[clap(long)]
    pub src: String,
    #[clap(long)]
    pub dest: String,
}

fn main() {
    let args = Args::parse();
    println!("Syncing {} to {}", args.src, args.dest);
    // TODO: call core library functions
}
