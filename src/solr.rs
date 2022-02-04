use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct GithubFile {
    pub id: String,
    pub file_id: String,
    pub owner_id: String,
    pub path: String,
    pub repo: String,
    pub branch: String,
    pub lang: String,
    pub content: Vec<String>,
}

pub async fn insert(data: &GithubFile, base_url: &str) -> Result<String, reqwest::Error> {
    let mut body: Vec<GithubFile> = Vec::new();
    body.push(data.clone());
    let url = format!("{}/solr/heline/update?&commitWithin=1000&overwrite=true&wt=json", base_url);
    let client = reqwest::Client::new();
    let res = client.post(url).json(&body).send().await?;
    let json = res.text().await?;
    Ok(json)
}
