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

pub async fn insert(data: &GithubFile) -> Result<String, reqwest::Error> {
    let mut body: Vec<GithubFile> = Vec::new();
    body.push(data.clone());
    // let url = "http://localhost:8984/solr/heline/update?&commitWithin=1000&overwrite=true&wt=json";
    let url = "https://heline.dev/solr/heline/update?&commitWithin=1000&overwrite=true&wt=json";
    let client = reqwest::Client::new();
    let res = client.post(url).json(&body).send().await?;
    let json = res.text().await?;
    Ok(json)
}
