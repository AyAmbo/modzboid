use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "modzboid")]
#[command(about = "Project Modzboid — Project Zomboid Mod Manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Subcommand)]
pub enum CliCommand {
    /// List all discovered mods
    ListMods {
        /// Path to workshop mods directory
        #[arg(short, long)]
        workshop: Option<String>,
    },
    /// Validate load order for a profile
    Validate {
        /// Profile name or ID
        #[arg(short, long)]
        profile: String,
    },
    /// Sort load order for a profile
    Sort {
        /// Profile name or ID
        #[arg(short, long)]
        profile: String,
    },
    /// Create a backup
    Backup {
        /// Backup name
        #[arg(short, long)]
        name: String,
    },
    /// List backups
    ListBackups,
    /// Show app version
    Version,
}

/// Check if CLI args indicate CLI mode (has a subcommand).
pub fn is_cli_mode() -> bool {
    let args: Vec<String> = std::env::args().collect();
    // If there are CLI-style args (not just the binary name), check for subcommands
    args.len() > 1 && !args[1].starts_with('-')
}

/// Run CLI command. Returns true if a command was executed.
pub fn run_cli() -> bool {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(_) => return false,
    };

    match cli.command {
        Some(CliCommand::Version) => {
            println!("Project Modzboid v0.1.0");
            true
        }
        Some(CliCommand::ListMods { workshop }) => {
            println!("Listing mods from: {}", workshop.as_deref().unwrap_or("(configured path)"));
            println!("(CLI mod listing requires app data — use the GUI for first setup)");
            true
        }
        Some(CliCommand::Validate { profile }) => {
            println!("Validating profile: {}", profile);
            println!("(CLI validation requires app data — use the GUI for first setup)");
            true
        }
        Some(CliCommand::Sort { profile }) => {
            println!("Sorting profile: {}", profile);
            println!("(CLI sorting requires app data — use the GUI for first setup)");
            true
        }
        Some(CliCommand::Backup { name }) => {
            println!("Creating backup: {}", name);
            println!("(CLI backup requires app data — use the GUI for first setup)");
            true
        }
        Some(CliCommand::ListBackups) => {
            println!("(CLI backup listing requires app data — use the GUI for first setup)");
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_version() {
        let cli = Cli::parse_from(["modzboid", "version"]);
        assert!(matches!(cli.command, Some(CliCommand::Version)));
    }

    #[test]
    fn test_cli_parse_list_mods() {
        let cli = Cli::parse_from(["modzboid", "list-mods", "--workshop", "/path"]);
        match cli.command {
            Some(CliCommand::ListMods { workshop }) => {
                assert_eq!(workshop, Some("/path".into()));
            }
            _ => panic!("Expected ListMods command"),
        }
    }

    #[test]
    fn test_cli_parse_no_command() {
        let cli = Cli::parse_from(["modzboid"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parse_backup() {
        let cli = Cli::parse_from(["modzboid", "backup", "--name", "test"]);
        match cli.command {
            Some(CliCommand::Backup { name }) => assert_eq!(name, "test"),
            _ => panic!("Expected Backup command"),
        }
    }
}
