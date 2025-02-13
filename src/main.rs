//! (1) Fetch the Completed Rewind Build (Breaking Commit) from Prometheus
//! (2) Post to Mastodon

use std::{
    fs::File,
    io::{BufReader, Write},
    thread::sleep,
    time::Duration,
};
use bit_vec::BitVec;
use clap::Parser;
use serde_json::{
    json,
    to_string_pretty,
    Value,
};
use url::Url;

// NuttX Target to be processed
const TARGET: &str = "rv-virt:knsh64_test8";

// Remembers the Mastodon Posts for All Builds:
// {
//   "rv-virt:citest" : {
//     status_id: "12345",
//     users: ["nuttxpr", "NuttX", "lupyuen"]
//   }
//   "rv-virt:citest64" : ...
// }
const ALL_BUILDS_FILENAME: &str = "/tmp/nuttx-rewind-notify.json";

/// Command-Line Arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init the Logger and Command-Line Args
    env_logger::init();
    let prometheus_server = std::env::var("PROMETHEUS_SERVER")
        .expect("PROMETHEUS_SERVER env variable is required");

    // Fetch the Breaking Commit from Prometheus
    let query = format!(r##"
        build_score{{
            target="{TARGET}",
            build_score_prev="1"
        }} == 0
    "##);
    println!("query={query}");
    let params = [("query", query)];
    let client = reqwest::Client::new();
    let prometheus = format!("http://{prometheus_server}/api/v1/query");
    let res = client
        .post(prometheus)
        .form(&params)
        .send()
        .await?;
    println!("res={res:?}");
    if !res.status().is_success() {
        println!("*** Prometheus Failed");
        sleep(Duration::from_secs(1));
    }
    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());
    let body = res.text().await?;
    // println!("Body: {body}");
    let data: Value = serde_json::from_str(&body).unwrap();
    let builds = &data["data"]["result"];
    // println!("\nbuilds={builds:?}");

    // Load the Mastodon Posts for All Builds
    let mut all_builds = json!({});
    if let Ok(file) = File::open(ALL_BUILDS_FILENAME) {
        let reader = BufReader::new(file);
        all_builds = serde_json::from_reader(reader).unwrap();
        // println!("\nall_builds=\n{}", to_string_pretty(&all_builds).unwrap());
    }

    // For Each Breaking Commit...
    for build in builds.as_array().unwrap() {
        // println!("build=\n{}", to_string_pretty(build).unwrap());
        let metric = &build["metric"];
        // println!("metric=\n{}", to_string_pretty(metric).unwrap());

        // Get the Previous NuttX Hash (Last Successful Commit)
        let nuttx_hash_prev = metric["nuttx_hash_prev"].as_str().unwrap();
        let url = metric["url"].as_str().unwrap();
        let board = metric["board"].as_str().unwrap();
        let config = metric["config"].as_str().unwrap();
        let user = metric["user"].as_str().unwrap();
        let config_upper = config.to_uppercase();
        let target = format!("{board}:{config}");
        println!("nuttx_hash_prev={nuttx_hash_prev}");
        println!("url={url}");
        println!("board={board}");
        println!("config={config}");
        println!("user={user}");
        // println!("msg=\n<<\n{msg}\n>>");

        // Get the Previous Log URL (Last Successful Commit)
        let previous_builds = search_builds_by_hash(nuttx_hash_prev).await?;
        let previous_build = &previous_builds[0];
        let previous_url = &previous_build["metric"]["url"].as_str().unwrap();
        // println!("previous_build=\n{previous_build:#}");
        println!("previous_url={previous_url}");

        // Get the Breaking PR from GitHub, based on the Breaking Commit
        // https://docs.github.com/en/rest/commits/commits?apiVersion=2022-11-28#list-pull-requests-associated-with-a-commit
        let client = reqwest::Client::new();
        let github = format!("https://api.github.com/repos/apache/nuttx/commits/{nuttx_hash_prev}/pulls");
        let res = client
            .get(github)
            .header("User-Agent", "nuttx-rewind-notify")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?;
        // println!("res={res:?}");
        if !res.status().is_success() {
            println!("*** GitHub Failed: {user} @ {target}");
            sleep(Duration::from_secs(30));
            continue;
        }
        // println!("Status: {}", res.status());
        // println!("Headers:\n{:#?}", res.headers());
        let body = res.text().await?;
        // println!("Body: {body}");
        let pull_requests: Value = serde_json::from_str(&body).unwrap();
        // println!("pull_requests=\n{pull_requests:#}");
        let pr = &pull_requests[0];
        let pr_url = pr["html_url"].as_str().unwrap();
        let pr_user = pr["user"]["login"].as_str().unwrap();
        println!("pr_url={pr_url}");
        println!("pr_user={pr_user}");

        // Read the Build Log

        // Extract the Build Log
        let mut log = extract_log(url).await?;
        log.insert(0, "```text".into());
        log.push("```".into());
        log.insert(0, url.into());
        log.insert(0, format!(
            r#"Sorry @{pr_user}: The above PR is failing for {TARGET}. Could you please take a look? Thanks!"#
        ));
        log.push(format!(
            r#"[(Earlier Commit is OK)]({previous_url})"#
        ));
        log.push(format!(
            r#"[(See the Build History)](https://nuttx-dashboard.org/d/fe2q876wubc3kc/nuttx-build-history?var-board={board}&var-config={config})"#
        ));
        // println!("log=\n{}", log.join("\n"));
        let msg = log.join("\n");

        // Compose the Mastodon Post as...
        // rv-virt : CITEST - Build Failed (NuttX)
        // NuttX Dashboard: ...
        // Build History: ...
        // [Error Message]
        let mut status = format!(
            r##"
{board} : {config_upper} - Build Failed ({user})
Breaking PR: {pr_url}
NuttX Dashboard: https://nuttx-dashboard.org
Build History: https://nuttx-dashboard.org/d/fe2q876wubc3kc/nuttx-build-history?var-board={board}&var-config={config}

{msg}
            "##);
        println!("status=\n{}", &status);

        // Upload the Complete Status as GitLab Snippet
        let snippet_url = create_snippet(&status).await?;
        println!("snippet_url=\n{}", &snippet_url);
        status.insert_str(0, &format!("{target} Rewind - {snippet_url}\n"));

        // Post the Truncated Status to Mastodon
        status.truncate(512);  // Mastodon allows only 500 chars
        let mut params = Vec::new();
        params.push(("status", status));

        // If the Mastodon Post already exists for Board and Config:
        // Reply to the Mastodon Post
        if let Some(status_id) = all_builds[&target]["status_id"].as_str() {
            params.push(("in_reply_to_id", status_id.to_string()));

            // If the User already exists for the Board and Config:
            // Skip the Mastodon Post
            if let Some(users) = all_builds[&target]["users"].as_array() {
                if users.contains(&json!(user)) {
                    println!("Skipping {user} @ {target}, already exists\n");
                    continue;
                }
            }
        }

        // Post to Mastodon
        let token = std::env::var("MASTODON_TOKEN")
            .expect("MASTODON_TOKEN env variable is required");
        let client = reqwest::Client::new();
        let mastodon = "https://nuttx-feed.org/api/v1/statuses";
        let res = client
            .post(mastodon)
            .header("Authorization", format!("Bearer {token}"))
            .form(&params)
            .send()
            .await?;
        println!("res={res:?}");
        if !res.status().is_success() {
            println!("*** Mastodon Failed: {user} @ {target}");
            sleep(Duration::from_secs(30));
            continue;
        }
        // println!("Status: {}", res.status());
        // println!("Headers:\n{:#?}", res.headers());
        let body = res.text().await?;
        println!("Body: {body}");

        // Remember the Mastodon Post ID (Status ID)
        let status: Value = serde_json::from_str(&body).unwrap();
        let status_id = status["id"].as_str().unwrap();
        println!("status_id={status_id}");
        all_builds[&target]["status_id"] = status_id.into();

        // Append the User to All Builds
        if let Some(users) = all_builds[&target]["users"].as_array() {
            if !users.contains(&json!(user)) {
                let mut users = users.clone();
                users.push(json!(user));
                all_builds[&target]["users"] = json!(users);
            }
        } else {
            all_builds[&target]["users"] = json!([user]);
        }

        // Save the Mastodon Posts for All Builds
        let json = to_string_pretty(&all_builds).unwrap();
        let mut file = File::create(ALL_BUILDS_FILENAME).unwrap();
        file.write_all(json.as_bytes()).unwrap();
        println!("\nall_builds=\n{json}\n");
        
        // Handle only the First Breaking Commit
        break;
    }

    // Return OK
    Ok(())
}

/// Extract the important bits from the Build / Test Log.
/// url looks like "https://gitlab.com/lupyuen/nuttx-build-log/-/snippets/4799962#L85"
async fn extract_log(url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // raw_url looks like "https://gitlab.com/lupyuen/nuttx-build-log/-/snippets/4799962/raw/"
    let parsed_url = Url::parse(url).unwrap();
    let start_line = parsed_url.fragment().unwrap();  // "L85"
    let start_linenum = start_line[1..].parse::<usize>().unwrap();  // 85
    let mut parsed_url = parsed_url.clone();
    parsed_url.set_fragment(None); // "https://gitlab.com/lupyuen/nuttx-build-log/-/snippets/4799962"
    let base_url = parsed_url.as_str();  
    let raw_url = format!("{base_url}/raw/");
    println!("raw_url={raw_url}");

    // output_line[i] is True if Line #i should be extracted for output (starts at i=1)
    let log = reqwest::get(raw_url).await?
        .text().await?;
    // println!("log=\n{log}");
    let lines = &log.split('\n').collect::<Vec<_>>();
    let mut output_line = BitVec::from_elem(lines.len() + 1, false);

    // Extract Log from Start Line Number till "===== Error: Test Failed" or "===== Test OK"
    for (linenum, line) in lines.into_iter().enumerate() {
        if linenum < start_linenum { continue; }
        if line.starts_with("===== ") {
            // Extract the previous 10 lines
            for i in (linenum - 10)..linenum { output_line.set(i, true); }
            // for i in (linenum - 10)..linenum { println!("{}", lines[i]); }
            break;
        } else if 
            // Skip these lines
            line.contains("/nuttx-build-farm/") ||  // "/home/luppy/nuttx-build-farm/build-test-knsh64.sh 657247bda89d60112d79bb9b8d223eca5f9641b5 a6b9e718460a56722205c2a84a9b07b94ca664aa"
            line.starts_with("+ [[") ||  // "[[ 657247bda89d60112d79bb9b8d223eca5f9641b5 != '' ]]"
            line.starts_with("+ set ") ||  // "set +x"
            line.starts_with("+ nuttx_hash") || // "nuttx_hash=657247bda89d60112d79bb9b8d223eca5f9641b5"
            line.starts_with("+ apps_hash") || // "apps_hash=a6b9e718460a56722205c2a84a9b07b94ca664aa"
            line.starts_with("+ neofetch") || // "neofetch"
            line.starts_with("+ tmp_path") || // "tmp_path=/tmp/build-test-knsh64"
            line.starts_with("+ rm -rf /tmp/") ||  // "rm -rf /tmp/build-test-knsh64"
            line.starts_with("+ mkdir /tmp/") ||  // "mkdir /tmp/build-test-knsh64"
            line.starts_with("+ cd /tmp/") ||  // "cd /tmp/build-test-knsh64"
            line.starts_with("+ riscv-none-elf-gcc -v") ||  // "riscv-none-elf-gcc -v"
            line.starts_with("+ rustup --version") ||  // "rustup --version"
            line.starts_with("+ rustc --version") ||  // "rustc --version"
            line.starts_with("+ riscv-none-elf-size") ||  // "riscv-none-elf-size nuttx"
            line.starts_with("+ script=") ||  // "script=qemu-riscv-knsh64"
            line.starts_with("+ wget ") ||  // "wget https://raw.githubusercontent.com/lupyuen/nuttx-riscv64/main/qemu-riscv-knsh64.exp"
            line.starts_with("+ expect ") ||  // "expect ./qemu-riscv-knsh64.exp"
            false {
            continue;
        } else if
            // Output these lines
            line.starts_with("+ ") ||
            line.starts_with("HEAD is now") ||  // "HEAD is now at 657247bda8 libc/modlib: preprocess gnu-elf.ld"
            line.starts_with("NuttX Source") ||  // "NuttX Source: https://github.com/apache/nuttx/tree/657247bda89d60112d79bb9b8d223eca5f9641b5"
            line.starts_with("NuttX Apps") ||  // "NuttX Apps: https://github.com/apache/nuttx-apps/tree/a6b9e718460a56722205c2a84a9b07b94ca664aa"
            line.contains("+ pushd ../apps") || // "CC:  ... + pushd ../apps"
            line.starts_with("spawn") ||  // "spawn qemu-system-riscv64 -semihosting -M virt,aclint=on -cpu rv64 -kernel nuttx -nographic"
            line.starts_with("QEMU emulator") ||  // "QEMU emulator version 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.4)"
            line.starts_with("OpenSBI") ||  // "OpenSBI v1.3"
            false {
            output_line.set(linenum, true);
            // println!("line={line}");
        }
    }

    // Consolidate the Extracted Log Lines
    let mut msg: Vec<String> = vec![];
    for (linenum, line) in lines.into_iter().enumerate() {
        if !output_line.get(linenum).unwrap() { continue; }
        let line =
            if line.contains("+ pushd ../apps") { "$ pushd ../apps".into() }  // "CC:  ... + pushd ../apps"
            else if line.starts_with("spawn ") { line.replace("spawn ", "$ ") }  // "spawn qemu-system-riscv64 -semihosting -M virt,aclint=on -cpu rv64 -kernel nuttx -nographic"
            else if line.starts_with("+ ") { "$ ".to_string() + &line[2..] }  // "+ " becomes "$ "
            else { line.to_string() };
        println!("{linenum}: {line}");
        msg.push(line);
    }
    Ok(msg)
}

// Search the NuttX Commit in Prometheus
async fn search_builds_by_hash(commit: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let prometheus_server = std::env::var("PROMETHEUS_SERVER")
        .expect("PROMETHEUS_SERVER env variable is required");
    let query = format!(r##"
        build_score{{
            target="{TARGET}",
            nuttx_hash="{commit}"
        }}
    "##);
    println!("query={query}");
    let params = [("query", query)];
    let client = reqwest::Client::new();
    let prometheus = format!("http://{prometheus_server}/api/v1/query");
    let res = client
        .post(prometheus)
        .form(&params)
        .send()
        .await?;
    println!("res={res:?}");
    if !res.status().is_success() {
        println!("*** Prometheus Failed");
        sleep(Duration::from_secs(1));
    }
    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());
    let body = res.text().await?;
    // println!("Body: {body}");
    let data: Value = serde_json::from_str(&body).unwrap();
    let builds = &data["data"]["result"];
    Ok(builds.clone())
}

// Create a GitLab Snippet. Returns the Snippet URL.
// https://docs.gitlab.com/ee/api/snippets.html#create-new-snippet
async fn create_snippet(content: &str) -> Result<String, Box<dyn std::error::Error>> {
    let user = "lupyuen";
    let repo = "nuttx-build-log";
    let body = r#"
{
  "title": "NuttX Rewind Notify",
  "description": "NuttX Breaking Commit",
  "visibility": "public",
  "files": [
    {
      "content": "Hello world",
      "file_path": "nuttx-rewind-notify.txt"
    }
  ]
}
    "#;
    let mut body: Value = serde_json::from_str(&body).unwrap();
    body["files"][0]["content"] = content.into();

    let token = std::env::var("GITLAB_TOKEN")
        .expect("GITLAB_TOKEN env variable is required");
    let client = reqwest::Client::new();
    let gitlab = format!("https://gitlab.com/api/v4/projects/{user}%2F{repo}/snippets");
    let res = client
        .post(gitlab)
        .header("Content-Type", "application/json")
        .header("PRIVATE-TOKEN", token)      
        .body(body.to_string())
        .send()
        .await?;
    // println!("res={res:?}");
    if !res.status().is_success() {
        println!("*** Create Snippet Failed: {user} @ {repo}");
        sleep(Duration::from_secs(30));
        panic!();
    }
    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());
    let response = res.text().await?;
    // println!("response={response}");
    let response: Value = serde_json::from_str(&response).unwrap();
    let url = response["web_url"].as_str().unwrap();
    Ok(url.into())
}
