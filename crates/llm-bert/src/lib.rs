mod sentiment;

use anyhow::Result;
pub use sentiment::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// BertAnalityze represents an entity that offers bert analitics.
pub trait BertAnalityze<'a, T>
where
    T: Debug + Serialize + Deserialize<'a>,
{
    /// Analyzes slice of texts.
    ///
    /// * `texts` - slice of texts to analyze.
    ///
    /// # Returns
    /// * Vector of results that are serializable or deserializable or error otherwise.
    fn analyze(&self, texts: &[String]) -> impl Future<Output = Result<Vec<T>>>;
}
