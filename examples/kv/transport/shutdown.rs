use http::uri::Authority;

use crate::node::Node;
use crate::tls::Tls;

impl Node {
    pub async fn shutdown(&self, tls: &Tls, peers: &[Authority]) {
        self.stop.cancel();
        self.tasks.close();
        self.tasks.wait().await;
        self.retire(tls, peers).await;
    }
}
