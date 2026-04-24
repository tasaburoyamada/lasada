use crate::core::traits::{ExecutionEngine, AppError, Result};
use async_trait::async_trait;
use scraper::{Html, Selector};
use log::{info, warn};

pub struct WebExecutor;

impl WebExecutor {
    pub fn new() -> Self {
        Self
    }

    async fn search(&self, query: &str) -> Result<String> {
        info!("Searching web for: {}", query);
        let client = reqwest::Client::new();
        // Using DuckDuckGo HTML version for simpler scraping
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
        
        let res = client.get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .send()
            .await
            .map_err(|e| AppError::ExecutionError(format!("Search request failed: {}", e)))?;

        let html_content = res.text().await
            .map_err(|e| AppError::ExecutionError(format!("Failed to read search response: {}", e)))?;

        let document = Html::parse_document(&html_content);
        let selector = Selector::parse(".result__body").unwrap();
        let title_selector = Selector::parse(".result__a").unwrap();
        let snippet_selector = Selector::parse(".result__snippet").unwrap();

        let mut results = String::new();
        results.push_str(&format!("Search results for: {}\n\n", query));

        for (i, element) in document.select(&selector).take(5).enumerate() {
            let title = element.select(&title_selector).next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();
            let link = element.select(&title_selector).next()
                .and_then(|e| e.value().attr("href"))
                .unwrap_or_default();
            let snippet = element.select(&snippet_selector).next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            results.push_str(&format!("{}. {}\n   URL: {}\n   Snippet: {}\n\n", i + 1, title.trim(), link, snippet.trim()));
        }

        if results.len() < 50 {
            warn!("Search results seem empty or blocked.");
            results.push_str("(No results found or access blocked by provider.)");
        }

        Ok(results)
    }

    async fn browse(&self, url: &str) -> Result<String> {
        info!("Browsing URL: {}", url);
        let client = reqwest::Client::new();
        let res = client.get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .send()
            .await
            .map_err(|e| AppError::ExecutionError(format!("Browse request failed: {}", e)))?;

        let html_content = res.text().await
            .map_err(|e| AppError::ExecutionError(format!("Failed to read page content: {}", e)))?;

        let document = Html::parse_document(&html_content);
        
        // Basic text extraction: strip tags, keep common text elements
        let body_selector = Selector::parse("body").unwrap();
        let mut text_content = String::new();

        if let Some(body) = document.select(&body_selector).next() {
            // Very simple text extraction
            for node in body.text() {
                let trimmed = node.trim();
                if !trimmed.is_empty() {
                    text_content.push_str(trimmed);
                    text_content.push(' ');
                }
            }
        }

        // Limit size
        if text_content.len() > 5000 {
            text_content.truncate(5000);
            text_content.push_str("... (Content truncated)");
        }

        Ok(text_content.trim().to_string())
    }
}

#[async_trait]
impl ExecutionEngine for WebExecutor {
    async fn start_session(&mut self) -> Result<()> {
        Ok(())
    }

    async fn execute(&mut self, code: &str, _language: &str) -> Result<String> {
        let code = code.trim();
        if code.starts_with("search ") {
            let query = &code[7..];
            self.search(query).await
        } else if code.starts_with("browse ") {
            let url = &code[7..];
            self.browse(url).await
        } else {
            Err(AppError::ExecutionError("Unknown web command. Use 'search <query>' or 'browse <url>'.".into()))
        }
    }
}
