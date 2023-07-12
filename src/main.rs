mod config;

use std::{env, sync::Arc};

use api::run_server;
use config::SparkConfig;
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use storage::{RelationDB, SmtManager};
use tx_builder::init_static_variables;

#[tokio::main]
async fn main() {
    let args = env::args().nth(1).expect("Missing env variable");
    let config: SparkConfig = config::parse_file(args).expect("Failed to parse config file");
    init_static_variables(
        config.network_type,
        config.metadata_type_id,
        config.checkpoint_type_id,
    );

    let ckb_rpc_client = Arc::new(CkbRpcClient::new(&config.ckb_node_url));
    let rdb = Arc::new(RelationDB::new(&config.rdb_url).await);
    let kvdb = Arc::new(SmtManager::new(&config.kvdb_path));

    let _handle = run_server(Arc::clone(&rdb), config.rpc_listen_address)
        .await
        .unwrap();

    println!("Hello, world!");
}
