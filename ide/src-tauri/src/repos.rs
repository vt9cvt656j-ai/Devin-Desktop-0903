//! Listing the repositories on a linked GitHub or GitLab account.
//!
//! The token is supplied by the client and used here, on this machine, talking directly to the
//! provider. It is never sent to our gateway — a personal access token is a credential for the
//! user's account, and routing it through a third party (even ours) would be the wrong default.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct Repository {
    pub full_name: String,
    pub private: bool,
    pub default_branch: String,
    pub url: String,
}

#[derive(Deserialize)]
struct GithubRepo {
    full_name: String,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    default_branch: String,
    #[serde(default)]
    html_url: String,
}

#[derive(Deserialize)]
struct GitlabProject {
    path_with_namespace: String,
    #[serde(default)]
    visibility: String,
    #[serde(default)]
    default_branch: String,
    #[serde(default)]
    web_url: String,
}

#[tauri::command]
pub async fn list_repositories(kind: String, token: String) -> Result<Vec<Repository>, String> {
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("尚未连接账号".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Michael-IDE")
        .build()
        .map_err(|e| e.to_string())?;

    match kind.as_str() {
        "github" => {
            let response = client
                .get("https://api.github.com/user/repos?per_page=100&sort=updated&affiliation=owner,collaborator,organization_member")
                .bearer_auth(&token)
                .header("Accept", "application/vnd.github+json")
                .send()
                .await
                .map_err(|e| format!("GitHub 请求失败：{e}"))?;
            if !response.status().is_success() {
                // The status is the useful part: 401 means the token is wrong, 403 usually means
                // it lacks the scope. "failed" would leave the user guessing which.
                return Err(format!("GitHub 返回 {}", response.status()));
            }
            let repos: Vec<GithubRepo> = response.json().await.map_err(|e| e.to_string())?;
            Ok(repos
                .into_iter()
                .map(|r| Repository {
                    full_name: r.full_name,
                    private: r.private,
                    default_branch: r.default_branch,
                    url: r.html_url,
                })
                .collect())
        }
        "gitlab" => {
            let response = client
                .get("https://gitlab.com/api/v4/projects?membership=true&per_page=100&order_by=last_activity_at")
                .header("PRIVATE-TOKEN", &token)
                .send()
                .await
                .map_err(|e| format!("GitLab 请求失败：{e}"))?;
            if !response.status().is_success() {
                return Err(format!("GitLab 返回 {}", response.status()));
            }
            let projects: Vec<GitlabProject> = response.json().await.map_err(|e| e.to_string())?;
            Ok(projects
                .into_iter()
                .map(|p| Repository {
                    private: p.visibility != "public",
                    full_name: p.path_with_namespace,
                    default_branch: p.default_branch,
                    url: p.web_url,
                })
                .collect())
        }
        other => Err(format!("不支持的代码托管平台：{other}")),
    }
}
