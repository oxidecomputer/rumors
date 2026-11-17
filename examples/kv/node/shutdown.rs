use super::Node;

impl Node {
    pub async fn shutdown(&self) {
        self.stop.cancel();
        self.tasks.close();
        self.tasks.wait().await;
        self.retire().await;
    }
}
