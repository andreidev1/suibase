use crate::network_monitor::NetmonMsg;

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use axum::http::header;
use reqwest::Method;

use crate::network_monitor::{NetMonRx, HEADER_SBSD_SERVER_HC, HEADER_SBSD_SERVER_IDX};

const SERVER_CHECK_REQUEST_BODY: &str =
    "{\"jsonrpc\":\"2.0\",\"method\":\"suix_getLatestSuiSystemState\",\"id\":1,\"params\":[\"\"]}";

pub struct RequestWorker {
    reqworker_rx: NetMonRx,
    client: reqwest::Client,
}

impl RequestWorker {
    pub fn new(reqworker_rx: NetMonRx) -> Self {
        Self {
            reqworker_rx,
            client: reqwest::Client::new(),
        }
    }

    async fn do_request(&mut self, msg: NetmonMsg) {
        let server_idx = msg.server_idx().to_string();

        let uri = format!("http://0.0.0.0:{}", msg.para16()[0]);
        let _ = self
            .client
            .request(Method::POST, uri)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::USER_AGENT, "curl/7.68.0")
            .header(header::ACCEPT, "*/*")
            .header(HEADER_SBSD_SERVER_IDX, server_idx.as_str())
            .header(HEADER_SBSD_SERVER_HC, "1")
            .body(SERVER_CHECK_REQUEST_BODY)
            .send()
            .await;

        //log::info!("do_request() msg {:?}", msg);

        // No error return here... never. Any failure of the request already
        // reflected by its execution by the proxy-server.
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = self.reqworker_rx.recv().await {
                // Process the message.
                self.do_request(msg).await;
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("shutting down - normal exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("shutting down - normal exit (1)");
                Ok(())
            }
        }
    }
}
