use super::Node;

impl Node {
    pub(crate) async fn shutdown(&self) {
        self.stop.cancel();
        self.tasks.close();
        self.tasks.wait().await;
        self.retire().await;
    }
}
