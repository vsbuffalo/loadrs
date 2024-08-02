use clap::Parser;
use colored::*;
use prettytable::color;
use prettytable::{format, Attr};
use prettytable::{Cell, Row, Table};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use sysinfo::{System, Users};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Excessive usage warning threshold (e.g. 80 = 80% usage)
    #[arg(short, long, default_value_t = 100.)]
    threshold: f64,
    /// When calculating fraction of active users, they are those with
    /// this *percent* usage (default is 1%).
    #[arg(short, long, default_value_t = 1.0)]
    active_threshold: f64,
    /// The user's fair share proportion (by default, calculated based
    /// total / number of active users, where an active user is set by
    /// --active-threshold.
    #[arg(short, long)]
    fair_share: Option<f64>,
    /// Update interval in seconds
    #[arg(short, long, default_value_t = 5)]
    interval: u64,
    /// Run in loop mode
    #[arg(short, long)]
    live: bool,
}

fn main() {
    let cli = Cli::parse();

    let (tx, rx) = mpsc::channel();

    ctrlc::set_handler(move || {
        tx.send(()).expect("Could not send signal on channel.");
    })
    .expect("Error setting Ctrl-C handler");

    loop {
        if cli.live {
            print!("\x1B[2J\x1B[1;1H");
        }

        let start_time = Instant::now();

        let mut sys = System::new_all();
        sys.refresh_all();
        let cpus = sys.cpus().len() as f64;

        // Create a mapping of user IDs to usernames
        let users = Users::new_with_refreshed_list();
        let uid_to_name: std::collections::HashMap<_, _> = users
            .iter()
            .map(|user| (user.id().to_string(), user.name().to_string()))
            .collect();

        let mut user_cpu_usage: Vec<(String, f64)> = sys
            .processes()
            .values()
            .filter_map(|p| {
                let username = p
                    .user_id()
                    .and_then(|uid| uid_to_name.get(&uid.to_string()).cloned())
                    .unwrap_or_else(|| {
                        format!(
                            "UID:{}",
                            p.user_id()
                                .map_or("Unknown".to_string(), |uid| uid.to_string())
                        )
                    });
                Some((username, p.cpu_usage()))
            })
            .fold(
                std::collections::HashMap::new(),
                |mut acc, (username, usage)| {
                    *acc.entry(username).or_insert(0.0) += usage as f64;
                    acc
                },
            )
            .into_iter()
            .collect();

        user_cpu_usage.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let active_users = user_cpu_usage
            .iter()
            .filter(|(_, usage)| *usage / cpus > cli.active_threshold)
            .count() as f64;
        let fair_share = cli.fair_share.unwrap_or(100.0 / active_users);

        // Print fair share information
        println!("\nFair Share Calculation:");
        if cli.fair_share.is_some() {
            println!("Using user-specified fair share: {:.2}%", fair_share);
        } else {
            println!("Using active users calculation:");
            println!(
                "  Active users (usage > {:.2}%): {}",
                cli.active_threshold, active_users
            );
            println!(
                "  Fair share = 100% / {} = {:.2}%\n",
                active_users, fair_share
            );
        }

        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(Row::new(vec![
            Cell::new("Username"),
            Cell::new("Total CPU Usage (%)"),
            Cell::new("Equivalent Cores Used"),
            Cell::new("System CPU Share (%)"),
        ]));

        for (user, sum) in &user_cpu_usage {
            if sum > &0.0 {
                let cpu_share = sum / cpus;
                let row_color = if cpu_share > fair_share {
                    "red".to_string()
                } else if cpu_share > fair_share * 0.5 {
                    "yellow".to_string()
                } else {
                    "green".to_string()
                };

                let colored_row = Row::new(vec![
                    Cell::new(&user)
                        .with_style(Attr::ForegroundColor(color_from_string(&row_color))),
                    Cell::new(&format!("{:.2}", sum))
                        .with_style(Attr::ForegroundColor(color_from_string(&row_color))),
                    Cell::new(&format!("{:.2}", sum / 100.0))
                        .with_style(Attr::ForegroundColor(color_from_string(&row_color))),
                    Cell::new(&format!("{:.2}", cpu_share))
                        .with_style(Attr::ForegroundColor(color_from_string(&row_color))),
                ]);

                table.add_row(colored_row);
            }
        }

        table.printstd();

        println!("\nTotal cores: {}", cpus as u32);
        let loadavg = System::load_average();
        println!("1 minute load average: {:.2}", loadavg.one);

        if loadavg.one > (cli.threshold / 100.) * cpus {
            println!("\n{}", "Excessive load detected!".red().bold());
            println!("Users exceeding fair share ({}%):", fair_share);
            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
            table.set_titles(Row::new(vec![
                Cell::new("Username"),
                Cell::new("System CPU Share (%)"),
                Cell::new("Excess Usage (%)"),
            ]));
            for (user, sum) in user_cpu_usage {
                if sum / cpus > fair_share {
                    table.add_row(Row::new(vec![
                        Cell::new(&user),
                        Cell::new(&format!("{:.2}%", sum / cpus)),
                        Cell::new(&format!("{:.2}%", (sum / cpus) - fair_share)),
                    ]));
                }
            }
            table.printstd();
        }

        if !cli.live {
            break;
        }

        let elapsed = start_time.elapsed();
        let sleep_duration = Duration::from_secs(cli.interval).saturating_sub(elapsed);

        if sleep_duration > Duration::from_millis(0) {
            match rx.recv_timeout(sleep_duration) {
                Ok(_) => {
                    println!("Received interrupt, exiting...");
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    break;
                }
            }
        }
    }

    println!("Exiting...");
}

fn color_from_string(color: &str) -> color::Color {
    match color {
        "red" => color::RED,
        "yellow" => color::YELLOW,
        "green" => color::GREEN,
        _ => color::WHITE,
    }
}
