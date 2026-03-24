use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorayProject {
    pub id: String,
    pub code: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wiki {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: String,
    pub subject: String,
    #[serde(default)]
    pub root: bool,
    /// Populated by our code after checking children
    #[serde(default)]
    pub has_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPageResult {
    pub id: String,
}

pub struct DoorayClient {
    client: Client,
    base_url: String,
    token: String,
}

impl DoorayClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        // Dooray API base is always https://api.dooray.com
        // but user might enter their org URL like https://org.dooray.com
        let api_base = if base_url.contains("api.dooray.com") {
            base_url.trim_end_matches('/').to_string()
        } else {
            "https://api.dooray.com".to_string()
        };

        Self {
            client: Client::new(),
            base_url: api_base,
            token: token.to_string(),
        }
    }

    pub async fn list_projects(&self) -> Result<Vec<DoorayProject>, String> {
        let url = format!("{}/project/v1/projects", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("dooray-api {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        #[derive(Deserialize)]
        struct ApiResponse {
            result: Vec<DoorayProject>,
        }

        let body: ApiResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(body.result)
    }

    pub async fn list_wikis(&self) -> Result<Vec<Wiki>, String> {
        let mut all_wikis = Vec::new();
        let mut page = 0;
        let size = 100;

        loop {
            let url = format!("{}/wiki/v1/wikis?page={}&size={}", self.base_url, page, size);
            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("dooray-api {}", self.token))
                .send()
                .await
                .map_err(|e| format!("Request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Wiki API error {}: {}", status, body));
            }

            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct ApiResponse {
                result: Vec<Wiki>,
                total_count: Option<i64>,
            }

            let body: ApiResponse = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            let count = body.result.len();
            all_wikis.extend(body.result);

            // Stop if we got less than page size or total reached
            if count < size || body.total_count.map_or(false, |t| all_wikis.len() as i64 >= t) {
                break;
            }
            page += 1;
        }

        // Sort alphabetically by name
        all_wikis.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(all_wikis)
    }

    pub async fn list_wiki_pages(
        &self,
        wiki_id: &str,
        parent_page_id: Option<&str>,
    ) -> Result<Vec<WikiPage>, String> {
        let mut all_pages = Vec::new();
        let mut page = 0;
        let size = 100;

        loop {
            let mut url = format!(
                "{}/wiki/v1/wikis/{}/pages?page={}&size={}",
                self.base_url, wiki_id, page, size
            );
            if let Some(pid) = parent_page_id {
                url.push_str(&format!("&parentPageId={}", pid));
            }

            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("dooray-api {}", self.token))
                .send()
                .await
                .map_err(|e| format!("Request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Wiki API error {}: {}", status, body));
            }

            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct ApiResponse {
                result: Vec<WikiPage>,
                total_count: Option<i64>,
            }

            let raw = resp.text().await
                .map_err(|e| format!("Failed to read response: {}", e))?;

            eprintln!("[dooray] wiki pages (wiki={}, parent={:?}, page={}): {}", wiki_id, parent_page_id, page, &raw[..raw.len().min(2000)]);

            let parsed: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;

            let result = parsed.get("result")
                .ok_or_else(|| format!("No 'result' in response: {}", &raw[..raw.len().min(500)]))?;

            let pages_batch: Vec<WikiPage> = serde_json::from_value(result.clone())
                .map_err(|e| format!("Failed to parse wiki pages: {}. Raw: {}", e, &raw[..raw.len().min(500)]))?;

            let total_count = parsed.get("totalCount")
                .and_then(|v| v.as_i64());

            let count = pages_batch.len();
            all_pages.extend(pages_batch);

            if count < size || total_count.map_or(false, |t| all_pages.len() as i64 >= t) {
                break;
            }
            page += 1;
        }

        all_pages.sort_by(|a, b| a.subject.to_lowercase().cmp(&b.subject.to_lowercase()));

        Ok(all_pages)
    }

    pub async fn create_wiki_page(
        &self,
        wiki_id: &str,
        parent_page_id: Option<&str>,
        title: &str,
        content: &str,
    ) -> Result<WikiPageResult, String> {
        let url = format!("{}/wiki/v1/wikis/{}/pages", self.base_url, wiki_id);

        let mut payload = serde_json::json!({
            "subject": title,
            "body": {
                "mimeType": "text/x-markdown",
                "content": content,
            },
        });

        if let Some(pid) = parent_page_id {
            payload["parentPageId"] = serde_json::Value::String(pid.to_string());
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("dooray-api {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        #[derive(Deserialize)]
        struct ApiResponse {
            result: WikiPageResult,
        }

        let response: ApiResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response.result)
    }

    pub async fn get_wiki_page_subject(
        &self,
        page_id: &str,
    ) -> Result<String, String> {
        let url = format!("{}/wiki/v1/pages/{}", self.base_url, page_id);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("dooray-api {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        #[derive(Deserialize)]
        struct PageResult {
            subject: String,
        }
        #[derive(Deserialize)]
        struct ApiResponse {
            result: PageResult,
        }

        let body: ApiResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(body.result.subject)
    }

    pub async fn update_wiki_page(
        &self,
        wiki_id: &str,
        page_id: &str,
        content: &str,
    ) -> Result<(), String> {
        // Fetch the existing page subject to preserve it
        let subject = self.get_wiki_page_subject(page_id).await?;

        let url = format!(
            "{}/wiki/v1/wikis/{}/pages/{}",
            self.base_url, wiki_id, page_id
        );

        let payload = serde_json::json!({
            "subject": subject,
            "body": {
                "mimeType": "text/x-markdown",
                "content": content,
            },
        });

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("dooray-api {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        Ok(())
    }
}
