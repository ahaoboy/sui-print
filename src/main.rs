use clap::{Parser, Subcommand};
use libsui::{Elf, Macho, PortableExecutable, find_section};
use std::env;
use std::error::Error;
use std::fs::File;

#[derive(Parser)]
#[command(name = "test")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Print {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Compile {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        args: Vec<String>,
        #[arg(long)]
        output: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let section_name = if cfg!(target_os = "windows") {
        "my_args"
    } else {
        "__MY_ARGS"
    };

    // Check for embedded data first
    if let Ok(Some(data)) = find_section(section_name) {
        println!("{}", String::from_utf8_lossy(data));
        return Ok(());
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Print { args }) => {
            println!("{}", args.join(" "));
        }
        Some(Commands::Compile { args, output }) => {
            // Remove the --output and its value from args if misplaced, but since clap handles it, args are before --output
            let data = args.join(" ").into_bytes();

            let self_path = env::current_exe()?;
            let exe_data = std::fs::read(&self_path)?;

            let mut out = File::create(&output)?;

            Elf::new(&exe_data).append(section_name, &data, &mut out)?;

            if cfg!(target_os = "macos") {
                Macho::from(exe_data)?
                    .write_section(section_name, data)?
                    .build(&mut out)?; // Use build_and_sign if code signing is required
            } else if cfg!(target_os = "windows") {
                PortableExecutable::from(&exe_data)?
                    .write_resource(section_name, data)?
                    .build(&mut out)?;
            } else if cfg!(target_os = "linux") {
                // Assuming Elf supports write_section; if not, implement append logic as per README
                Elf::new(&exe_data).append(section_name, &data, &mut out)?
            } else {
                return Err("Unsupported operating system".into());
            }

            // Set executable permissions on Unix-like systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&output)?;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);
                std::fs::set_permissions(&output, permissions)?;
            }

            println!("Generated executable: {}", output);
        }
        None => {
            println!("Usage: test [args|compile] ...");
        }
    }

    Ok(())
}
