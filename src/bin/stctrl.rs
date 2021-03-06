#![feature(futures_api, async_await, await_macro, arbitrary_self_types)]
#![feature(nll)]
#![feature(generators)]
#![feature(never_type)]

#![deny(
    trivial_numeric_casts,
    warnings
)]

#[macro_use] extern crate log;

// use std::convert::TryInto;
use std::path::PathBuf;
use std::env;

use log::Level;

use futures::executor::ThreadPool;

use clap::{Arg, App, AppSettings, SubCommand /*, ArgMatches */};

use stctrl::info::{info, InfoError};
use stctrl::config::{config, ConfigError};
use stctrl::funds::{funds, FundsError};

use app::{connect, identity_from_file, load_node_from_file};


const STCTRL_ID_FILE: &str = "STCTRL_ID_FILE";
const STCTRL_NODE_TICKET_FILE: &str = "STCTRL_NODE_TICKET_FILE";


#[derive(Debug)]
enum StCtrlError {
    CreateThreadPoolError,
    MissingIdFileArgument,
    IdFileDoesNotExist,
    MissingNodeTicketArgument,
    NodeTicketFileDoesNotExist,
    InvalidNodeTicketFile,
    SpawnIdentityServiceError,
    ConnectionError,
    InfoError(InfoError),
    ConfigError(ConfigError),
    FundsError(FundsError),
}


impl From<InfoError> for StCtrlError {
    fn from(e: InfoError) -> Self {
        StCtrlError::InfoError(e)
    }
}

impl From<ConfigError> for StCtrlError {
    fn from(e: ConfigError) -> Self {
        StCtrlError::ConfigError(e)
    }
}

impl From<FundsError> for StCtrlError {
    fn from(e: FundsError) -> Self {
        StCtrlError::FundsError(e)
    }
}

/// Get environment variable
fn get_env(key: &str) -> Option<String> {
    for (cur_key, value) in env::vars() {
        if cur_key == key {
            return Some(value);
        }
    }
    None
}

/// Get stctrl id file path by reading an environment variable
fn env_stctrl_id_file() -> Option<PathBuf> {
    Some(PathBuf::from(get_env(STCTRL_ID_FILE)?))
}

/// Get stctrl node ticket file path by reading an environment variable
fn env_stctrl_node_ticket_file() -> Option<PathBuf> {
    Some(PathBuf::from(get_env(STCTRL_NODE_TICKET_FILE)?))
}


fn run() -> Result<(), StCtrlError> {

    simple_logger::init_with_level(Level::Warn).unwrap();
    let mut thread_pool = ThreadPool::new()
        .map_err(|_| StCtrlError::CreateThreadPoolError)?;

    let matches = App::new("stctrl: offST ConTRoL")
                          // TOOD: Does this setting work for recursive subcommands?
                          .setting(AppSettings::SubcommandRequiredElseHelp)
                          .version("0.1.0")
                          .author("real <real@freedomlayer.org>")
                          .about("A command line client for offst node")
                          // STCTRL_ID_FILE
                          .arg(Arg::with_name("idfile")
                               .short("I")
                               .long("idfile")
                               .value_name("idfile")
                               .help("Client identity file path")
                               .required(false))
                          // STCTRL_NODE_TICKET_FILE
                          .arg(Arg::with_name("node_ticket")
                               .short("T")
                               .long("ticket")
                               .value_name("node_ticket")
                               .help("Node ticket file path")
                               .required(false))

                          /* ------------[Info] ------------- */
                          .subcommand(SubCommand::with_name("info")
                              .about("show offst node information")
                              .subcommand(SubCommand::with_name("relays")
                                  .about("Show all configured relays"))

                              .subcommand(SubCommand::with_name("index")
                                  .about("Show all configured index servers"))

                              .subcommand(SubCommand::with_name("friends")
                                  .about("Show all configured friends"))

                              .subcommand(SubCommand::with_name("last-friend-token")
                                  .about("Last received token from this friend")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("balance")
                                  .about("Display current balance"))

                              .subcommand(SubCommand::with_name("export-ticket")
                                  .about("Export a ticket of this node's contact information")
                                  .arg(Arg::with_name("output_file")
                                       .short("o")
                                       .long("output")
                                       .value_name("output_file")
                                       .help("output node ticket file path")
                                       .required(true))))

                          /* ------------[Config] ------------- */
                          .subcommand(SubCommand::with_name("config")
                              .about("configure offst node")
                              .subcommand(SubCommand::with_name("add-relay")
                                  .about("Add a relay")
                                  .arg(Arg::with_name("relay_file")
                                       .short("r")
                                       .long("relay")
                                       .value_name("relay_file")
                                       .help("relay file")
                                       .required(true))
                                  .arg(Arg::with_name("relay_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("relay_name")
                                       .help("relay name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("remove-relay")
                                  .about("Remove a relay")
                                  .arg(Arg::with_name("relay_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("relay_name")
                                       .help("relay name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("add-index")
                                  .about("Add an index server")
                                  .arg(Arg::with_name("index_file")
                                       .short("x")
                                       .long("index")
                                       .value_name("index_file")
                                       .help("index file")
                                       .required(true))
                                  .arg(Arg::with_name("index_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("index_name")
                                       .help("Index server name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("remove-index")
                                  .about("Remove an index server")
                                  .arg(Arg::with_name("index_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("index_name")
                                       .help("Index server name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("add-friend")
                                  .about("Add a friend")
                                  .arg(Arg::with_name("friend_file")
                                       .short("f")
                                       .long("friend")
                                       .value_name("friend_file")
                                       .help("friend file")
                                       .required(true))
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend name")
                                       .required(true))
                                  .arg(Arg::with_name("friend_balance")
                                       .short("b")
                                       .long("balance")
                                       .value_name("friend_balance")
                                       .help("Initial balance with friend")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("set-friend-relays")
                                  .about("Set a friend's relays")
                                  .arg(Arg::with_name("friend_file")
                                       .short("f")
                                       .long("friend")
                                       .value_name("friend_file")
                                       .help("friend file")
                                       .required(true))
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("remove-friend")
                                  .about("Remove a friend\
                                          Caution: This is a violent operation.")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend's name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("enable-friend")
                                  .about("Enable a friend")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend's name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("disable-friend")
                                  .about("Disable a friend")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend's name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("open-friend")
                                  .about("Open a friend")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend's name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("close-friend")
                                  .about("Close a friend")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend's name")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("set-friend-max-debt")
                                  .about("Set friend's max debt")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend's name")
                                       .required(true))
                                  .arg(Arg::with_name("max_debt")
                                       .short("m")
                                       .long("mdebt")
                                       .value_name("max_debt")
                                       .help("Max debt value")
                                       .required(true)))

                              .subcommand(SubCommand::with_name("reset-friend")
                                  .about("Reset mutual credit with friend according to the friend's terms")
                                  .arg(Arg::with_name("friend_name")
                                       .short("n")
                                       .long("name")
                                       .value_name("friend_name")
                                       .help("friend's name")
                                       .required(true))))

                          /* ------------[Funds] ------------- */
                          .subcommand(SubCommand::with_name("funds")
                              .about("configure offst node")
                              .subcommand(SubCommand::with_name("send")
                                  .about("Send funds to a remote destination")
                                  .arg(Arg::with_name("destination")
                                       .short("d")
                                       .long("destination")
                                       .value_name("destination")
                                       .help("recipient's public key")
                                       .required(true))
                                  .arg(Arg::with_name("amount")
                                       .short("a")
                                       .long("amount")
                                       .value_name("amount")
                                       .help("Amount of credits to send")
                                       .required(true))))
                          .get_matches();

    // Get application's identity:
    let idfile_pathbuf = match matches.value_of("idfile") {
        Some(idfile) => PathBuf::from(idfile),
        None => {
            if let Some(idfile_pathbuf) = env_stctrl_id_file() {
                idfile_pathbuf
            } else {
                return Err(StCtrlError::MissingIdFileArgument);
            }
        },
    };

    if !idfile_pathbuf.exists() {
        return Err(StCtrlError::IdFileDoesNotExist);
    }

    // Get node's connection information (node-ticket):
    let node_ticket_pathbuf = match matches.value_of("node_ticket") {
        Some(node_ticket) => PathBuf::from(node_ticket),
        None => {
            if let Some(node_ticket_pathbuf) = env_stctrl_node_ticket_file() {
                node_ticket_pathbuf
            } else {
                return Err(StCtrlError::MissingNodeTicketArgument);
            }
        },
    };

    if !node_ticket_pathbuf.exists() {
        return Err(StCtrlError::NodeTicketFileDoesNotExist);
    }

    // Get node information from file:
    let node_address = load_node_from_file(&node_ticket_pathbuf)
        .map_err(|_| StCtrlError::InvalidNodeTicketFile)?;

    // Spawn identity service:
    let app_identity_client = identity_from_file(&idfile_pathbuf, thread_pool.clone())
        .map_err(|_| StCtrlError::SpawnIdentityServiceError)?;


    let c_thread_pool = thread_pool.clone();
    thread_pool.run(async move {
        // Connect to node:
        let node_connection = await!(connect(node_address.public_key,
                            node_address.address,
                            app_identity_client,
                            c_thread_pool.clone()))
            .map_err(|_| StCtrlError::ConnectionError)?;

        Ok(match matches.subcommand() {
            ("info", Some(matches)) => await!(info(matches, node_connection))?,
            ("config", Some(matches)) => await!(config(matches, node_connection))?,
            ("funds", Some(matches)) => await!(funds(matches, node_connection))?,
            _ => unreachable!(),
        })
    })
}

fn main() {
    if let Err(e) = run() {
        error!("run() error: {:?}", e);
    }
}
