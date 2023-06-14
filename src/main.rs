use std::env;
use std::io::{self, stdin, stdout, Write};
use std::process::{exit, Child, Command, Stdio};
struct Config {
    prompt: String,
    version: String,
}

impl Default for Config {
    fn default() -> Self {
        Config { prompt: ">".into(), version: "0.1".into() }
    }
}

fn main() {
    // create a config structure from default implementation
    let config = Config::default();
    //loop indefinitely waiting for user input through stdin
    loop {
        if let Err(e) = prompter(&config) {
            eprintln!("{}", e);
            return;
        }
    }
}

// get user input through stdin and parse it 
fn prompter(config: &Config) -> io::Result<()> {
    print!("{} ", config.prompt);
    stdout().flush().expect("Could not flush stdout."); // for printing the prompt immediately.

    let mut buffer = String::new();
    stdin().read_line(&mut buffer)?;
    parse_stdin(&buffer, config);
    Ok(())
}

fn parse_stdin(buffer: &str, config: &Config) {
    // parse the buffer, split for each pipe
    let mut commands = buffer.trim().split('|').peekable();
    // create a variable for previous command, 
    // if there is one we will connect previous
    // command's stdout and current command's stdin 
    let mut previous_command: Option<Child> = None;
    // while there are still commands, consume each command one by one
    while let Some(command) = commands.next() {
        // parse each command, first argument is the program itself, the rest is arguments.
        let mut args = command.split_whitespace();
        // if user just presses enter, just skip to the next iteration
        let program = args.next().unwrap_or("skip");
        // check for special programs
        match program {
            // skip an iteration and print the prompt again
            "skip" => {
                return;
            }
            //exit the shell
            "exit" => {
                exit(0);
            }
            //print version
            "version" => {
                println!("{}",config.version);
            }
            //change current shell, cd should be shell built-in
            // because it changes internals of the shell,
            // we can't just use Command::new("cd")... since that means we run another process
            // read: https://stackoverflow.com/a/31897001/15741574
            // also: https://unix.stackexchange.com/questions/38808/why-is-cd-not-a-program/38819#38819
            "cd" => {
                // check if there is a home directory, if not use the root directory as home
                let home = env::var("HOME").unwrap_or("/".into());
                let new_dir = args.next().unwrap_or(&home);
                if let Err(e) = env::set_current_dir(new_dir) {
                    eprintln!("{}", e);
                }
                // cd doesn't accept arguments through stdin, therefore we don't care about the previous command
                // and we make it None because cd won't pipe anything to stdout.
                previous_command = None;
            }
            //usual programs,
            program => {
                // get stdin through pipe if there was a previous command, else inherit from parent.
                let input = previous_command.map_or(Stdio::inherit(), |output| {
                    Stdio::from(output.stdout.unwrap())
                });
                // if there is another command next, we connect our stdout to their stdin
                let output = if commands.peek().is_some() {
                    //there is another command
                    Stdio::piped()
                } else {
                    //we are the final command
                    Stdio::inherit()
                };
                //run the command with specified configuration
                let output = Command::new(program)
                    .args(args)
                    .stdin(input)
                    .stdout(output)
                    .spawn();
                // save the current command and go to the next command in buffer, 
                // this helps us when checking the final command because we will have to wait
                // for the last command in order to get it's results without going to the next line in terminal 
                match output {
                    Ok(child) => {
                        previous_command = Some(child); // save current command
                    }
                    Err(e) => {
                        previous_command = None; // there was an error, don't save the commands
                        eprintln!("{}", e);
                    }
                }
            }
        }
    }
    // wait for the last command before getting another input for another command
    if let Some(mut last_command) = previous_command {
        // check exit status
        match last_command.wait() {
            Ok(exit_status) => {
                // check if there was an error.
                if !exit_status.success() {
                    // command ran but there was an error
                    // default to 1 if there was no error status
                    eprint!("{} ", exit_status.code().unwrap_or(1));
                }
            }
            // command couldn't run
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}
