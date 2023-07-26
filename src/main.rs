mod config;

use std::{
    env,
    panic::PanicInfo,
    sync::{atomic::AtomicU64, Arc},
};

use api::run_server;
use backtrace::Backtrace;
use ckb_types::H256;
use config::SparkConfig;
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use storage::{RelationDB, SmtManager, KVDB};
use sync::Synchronization;
#[cfg(unix)]
use tokio::signal::unix as os_impl;
use tx_builder::init_static_variables;

#[tokio::main]
async fn main() {
    init_log();
    // let args = env::args().nth(0).unwrap_or("./config.toml".to_string());
    let config: SparkConfig =
        config::parse_file("./config.toml").expect("Failed to parse config file");
    init_static_variables(
        config.network_type.clone(),
        config.axon_token_type_args.as_bytes().to_vec().into(),
        config.xudt_owner.clone(),
        config.issuance_type_id.clone(),
        config.metadata_type_id.clone(),
        config.metadata_code_hash.clone(),
        config.checkpoint_type_id.clone(),
        config.stake_at_code_hash.clone(),
        config.stake_smt_code_hash.clone(),
        config.delegate_at_code_hash.clone(),
        config.delegate_smt_code_hash.clone(),
    );

    let ckb_rpc_client = Arc::new(CkbRpcClient::new(&config.ckb_node_url));
    let rdb = Arc::new(RelationDB::new(&config.rdb_url).await);
    let stake_smt = Arc::new(SmtManager::new(&config.stake_smt_db()));
    let delegate_smt = Arc::new(SmtManager::new(&config.delegate_smt_db()));
    let reward_smt = Arc::new(SmtManager::new(&config.reward_smt_db()));
    let kvdb = Arc::new(KVDB::new(&config.status_db()));
    let current_epoch = Arc::new(AtomicU64::new(kvdb.get_current_epoch().await.unwrap()));

    let sync = Synchronization::new(
        Arc::clone(&ckb_rpc_client),
        Arc::clone(&rdb),
        Arc::clone(&kvdb),
        stake_smt,
        delegate_smt,
        reward_smt,
        config.start_number,
        Arc::clone(&current_epoch),
        H256::from_trimmed_str(&config.private_key[2..]).unwrap(),
    )
    .await;

    tokio::spawn(async move {
        sync.run().await;
    });

    let _handle = run_server(
        rdb,
        kvdb,
        ckb_rpc_client,
        current_epoch,
        config.rpc_listen_address,
    )
    .await
    .unwrap();

    set_ctrl_c_handle().await;
}

async fn set_ctrl_c_handle() {
    let ctrl_c_handler = tokio::spawn(async {
        #[cfg(windows)]
        let _ = tokio::signal::ctrl_c().await;
        #[cfg(unix)]
        {
            let mut sigtun_int = os_impl::signal(os_impl::SignalKind::interrupt()).unwrap();
            let mut sigtun_term = os_impl::signal(os_impl::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigtun_int.recv() => {}
                _ = sigtun_term.recv() => {}
            };
        }
    });

    // register channel of panic
    let (panic_sender, mut panic_receiver) = tokio::sync::mpsc::channel::<()>(1);

    std::panic::set_hook(Box::new(move |info: &PanicInfo| {
        let panic_sender = panic_sender.clone();
        panic_log(info);
        panic_sender.try_send(()).expect("panic_receiver is droped");
    }));

    tokio::select! {
        _ = ctrl_c_handler => { log::info!("ctrl + c is pressed, quit.") },
        _ = panic_receiver.recv() => { log::info!("child thread panic, quit.") },
    };
}

fn panic_log(info: &PanicInfo) {
    let backtrace = Backtrace::new();
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("unnamed");
    let location = info.location().unwrap(); // The current implementation always returns Some
    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &**s,
            None => "Box<Any>",
        },
    };
    log::error!(
        target: "panic", "thread '{}' panicked at '{}': {}:{} {:?}",
        name,
        msg,
        location.file(),
        location.line(),
        backtrace,
    );
}

fn init_log() {
    let mut builder = env_logger::builder();
    builder.filter_level(log::LevelFilter::Info);
    builder.filter(Some("sqlx"), log::LevelFilter::Info);
    builder.init();

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_target(true)
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
