use crate::BertAnalityze;
use anyhow::{Error, Result};
use rust_bert::pipelines::sentiment::{Sentiment, SentimentConfig, SentimentModel};
use std::sync::mpsc;
use tokio::{
    sync::oneshot,
    task::{self, JoinHandle},
};

const CHANNELS_COUNT: usize = 100;

type Message = (Vec<String>, oneshot::Sender<Vec<Sentiment>>);

/// Runner for sentiment classification

#[derive(Debug, Clone)]
pub struct SentimentClassifier {
    sender: mpsc::SyncSender<Message>,
}

impl SentimentClassifier {
    pub fn spawn() -> (JoinHandle<Result<(), String>>, SentimentClassifier) {
        let (sender, receiver) = mpsc::sync_channel(CHANNELS_COUNT);
        let handle = task::spawn_blocking(move || Self::run(receiver));
        (handle, SentimentClassifier { sender })
    }

    fn run(receiver: mpsc::Receiver<Message>) -> Result<(), String> {
        let model = SentimentModel::new(SentimentConfig::default()).map_err(|e| e.to_string())?;

        while let Ok((texts, sender)) = receiver.recv() {
            let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
            let sentiments = model.predict(texts);
            sender.send(sentiments).expect("sending results");
        }

        Ok(())
    }
}

impl<'a> BertAnalityze<'a, Sentiment> for SentimentClassifier {
    async fn analyze(&self, texts: &[String]) -> Result<Vec<Sentiment>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((texts.to_vec(), sender))
            .map_err(Error::from)?;
        receiver.await.map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_should_predict_sentiment() -> Result<()> {
        use super::*;

        let (_handle, classifier) = SentimentClassifier::spawn();

        let texts = vec![
            "Analysts forecast 2025 targets between $70,000 and $250,000, contingent on ETF flows, Fed policies, and regulatory developments.".to_owned(),
            "The momentum in the Bitcoin market is also driving gains in altcoins. Ethereum (ETH) and XRP are up by approximately 4%, while Solana (SOL) and Dogecoin (DOGE) have gained over 5%.".to_owned(),
        ];
        let sentiments = classifier.analyze(&texts).await?;
        for (i, sentiment) in sentiments.iter().enumerate() {
            println!("Result_{i}:  {sentiment:?}");
        }
        Ok(())
    }
}
