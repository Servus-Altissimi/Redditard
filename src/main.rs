//                  ,--.   ,--.,--.  ,--.                                    
// ,--.--. ,---.  ,-|  | ,-|  |`--',-'  '-.    ,--.,--. ,---.  ,---. ,--.--. 
// |  .--'| .-. :' .-. |' .-. |,--.'-.  .-'    |  ||  |(  .-' | .-. :|  .--' 
// |  |   \   --.\ `-' |\ `-' ||  |  |  |      '  ''  '.-'  `)\   --.|  |    
// `--'    `----' `---'  `---' `--'  `--'       `----' `----'  `----'`--'    

// Requires Ollama & Chromedriver
// Uses reddit for you!  
// I neither care nor am responsible for any damages. 

// Copyright 2025 Servus Altissimi (Pseudonym)

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.                                                                          

use thirtyfour::prelude::*;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use tokio::time::{sleep, Duration};
use anyhow::{Result, Context};
use rand::Rng;
use std::collections::HashSet;
use std::fs;
use std::io::{Write, BufRead, BufReader, stdin};
use serde::Deserialize;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Reddit comment bot with natural behaviour", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "deepseek-r1:latest")]
    model: String,

    #[arg(short = 'H', long)]
    headless: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    upvote: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct SubredditConfig {
    name: String,
    sort: String,
    #[serde(default)]
    timeframe: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Config {
    subreddits: Vec<SubredditConfig>,
}

#[derive(Debug, Deserialize)]
struct PromptConfig {
    #[serde(default)]
    custom_prompt: Option<String>,
}

struct RedditBot {
    driver: WebDriver,
    ollama: Ollama,
    username: String,
    password: String,
    commented_posts: HashSet<String>,
    config: Config,
    comment_count: u32,
    model: String,
    verbose: bool,
    upvote_enabled: bool,
    prompt_template: String,
}

impl RedditBot {
    async fn new(username: String, password: String, args: &Args) -> Result<Self> {
        let config_str = fs::read_to_string("subreddits.toml")
            .context("Failed to read subreddits.toml")?;
        let config: Config = toml::from_str(&config_str)
            .context("Failed to parse subreddits.toml")?;

        let prompt_template = Self::load_prompt_template(args.verbose)?;
        let commented_posts = Self::load_posted_history()?;
        
        if args.verbose {
            println!("[INIT] Loaded {} previously commented posts", commented_posts.len());
            println!("[INIT] Using {} prompt", if prompt_template.contains("{{SUBREDDIT}}") { "custom" } else { "default" });
        }

        let mut caps = DesiredCapabilities::chrome();
        
        let user_agents = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 6.1; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_6) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 13; SM-G991B) AppleWebKit/537.36",
            "Mozilla/5.0 (iPad; CPU OS 16_6 like Mac OS X) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 12_5_1) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; Fedora; Linux x86_64) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 12; OnePlus 9) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 11; Nokia X20) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 6.3; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; CrOS x86_64 15604.45.0) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 10.0) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_13_6) AppleWebKit/537.36",
        ];
        
        let mut rng = rand::thread_rng();
        let user_agent = user_agents[rng.gen_range(0..user_agents.len())];
        
        let mut chrome_args = vec![
            format!("--user-agent={}", user_agent),
            "--disable-blink-features=AutomationControlled".to_string(),
            "--disable-dev-shm-usage".to_string(),
            "--no-sandbox".to_string(),
            "--disable-gpu".to_string(),
            "--disable-infobars".to_string(),
            "--start-maximized".to_string(),
            "--disable-notifications".to_string(),
            "--disable-popup-blocking".to_string(),
            "--disable-extensions".to_string(),
            "--lang=en-US".to_string(),
        ];
        
        if args.headless {
            chrome_args.push("--headless=new".to_string());
            chrome_args.push("--window-size=1920,1080".to_string());
            chrome_args.push("--virtual-display-pixel-depth=24".to_string());
        }
        
        for arg in &chrome_args {
            caps.add_arg(arg)?;
        }
        
        caps.add_experimental_option("excludeSwitches", vec!["enable-automation"])?;
        caps.add_experimental_option("useAutomationExtension", false)?;
        
        let driver = WebDriver::new("http://localhost:9517", caps)
            .await
            .context("Failed to create WebDriver. Make sure ChromeDriver is running on port 9517")?;
        
        let _ = driver.execute(
            "const newProto = navigator.__proto__; delete newProto.webdriver; navigator.__proto__ = newProto;",
            vec![]
        ).await;
        
        if args.verbose {
            println!("[INIT] Using User-Agent: {}", user_agent);
        }
        
        let ollama = Ollama::default();
        
        Ok(Self {
            driver,
            ollama,
            username,
            password,
            commented_posts,
            config,
            comment_count: 0,
            model: args.model.clone(),
            verbose: args.verbose,
            upvote_enabled: args.upvote,
            prompt_template,
        })
    }

    fn load_prompt_template(verbose: bool) -> Result<String> {
        match fs::read_to_string("prompt.toml") {
            Ok(content) => {
                match toml::from_str::<PromptConfig>(&content) {
                    Ok(prompt_config) => {
                        if let Some(custom) = prompt_config.custom_prompt {
                            if verbose {
                                println!("[PROMPT] Loaded custom prompt from prompt.toml");
                            }
                            return Ok(custom);
                        }
                    }
                    Err(e) => {
                        if verbose {
                            println!("[PROMPT] Warning: Failed to parse prompt.toml: {}", e);
                            println!("[PROMPT] Using default prompt");
                        }
                    }
                }
            }
            Err(_) => {
                if verbose {
                    println!("[PROMPT] No prompt.toml found, using default prompt");
                }
            }
        }

        // to be safe
        Ok(r#"You're browsing r/{{SUBREDDIT}} and just saw this post. Write a quick 1-2 sentence reaction that sounds like an actual person.

Post title: {{TITLE}}{{BODY_CONTEXT}}

STYLE:
- Punch
- Actually excited or interested, not fake hype
- Say what you think, share what you know, or just react
- Talk like a normal person texting a friend
- Be chill, funny, helpful, or hype WHEN ITS DESERVED

- Try to be a little cute, not a grown man
DO:
- Get straight to the point
- Share quick personal experience if relevant
- Drop useful info casually
- Use natural slang: tbh, fr, ngl, lowkey, deadass (but dont overdo it)
- Sometimes just be like damn thats cool or yoo nice
- Match the energy of the post{{EMOTICON_INSTRUCTION}}

DONT:
- Ask rhetorical questions or any questions unless you genuinely wanna know
- Sound like an AI or corporate bot
- Force jokes or use cringe puns
- Use hashtags or multiple exclamation points
- Be overly nice or overly enthusiastic
- Use dashes or semicolons for no reason

Just write the comment. Nothing else. NO quotation marks:"#.to_string())
    }

    fn load_posted_history() -> Result<HashSet<String>> {
        let mut posted = HashSet::new();
        
        if let Ok(file) = fs::File::open("posted.txt") {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(post_id) = line {
                    let post_id = post_id.trim();
                    if !post_id.is_empty() && post_id.contains('|') {
                        if let Some(id) = post_id.split('|').next() {
                            posted.insert(id.trim().to_string());
                        }
                    }
                }
            }
        }
        
        Ok(posted)
    }

    fn save_posted(&self, post_id: &str, subreddit: &str, title: &str) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("posted.txt")?;
        
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(file, "{} | {} | r/{} | {}", post_id, timestamp, subreddit, title)?;
        
        if self.verbose {
            println!("[LOG] Saved to posted.txt");
        }
        Ok(())
    }

    async fn save_screenshot(&self, filename: &str) -> Result<()> {
        let screenshot = self.driver.screenshot_as_png().await?;
        std::fs::write(filename, screenshot)?;
        if self.verbose {
            println!("[SCREENSHOT] Saved: {}", filename);
        }
        Ok(())
    }

    async fn human_scroll(&self) {
        let mut rng = rand::thread_rng();
        let scroll_amount = rng.gen_range(100..500);
        let _ = self.driver.execute(
            &format!("window.scrollBy(0, {});", scroll_amount),
            vec![]
        ).await;
        sleep(Duration::from_millis(rng.gen_range(200..800))).await;
    }

    async fn human_type(&self, element: &WebElement, text: &str) -> Result<()> {
        let mut rng = rand::thread_rng();
        element.click().await?;
        sleep(Duration::from_millis(rng.gen_range(50..200))).await;
        
        for ch in text.chars() {
            element.send_keys(&ch.to_string()).await?;
            let delay = if ch == ' ' {
                rng.gen_range(30..80)
            } else {
                rng.gen_range(20..60)
            };
            sleep(Duration::from_millis(delay)).await;
        }
        Ok(())
    }

    async fn random_pause(&self) {
        let mut rng = rand::thread_rng();
        sleep(Duration::from_millis(rng.gen_range(500..2000))).await;
    }

    async fn handle_cookie_popup(&self) {
        sleep(Duration::from_secs(5)).await;
        
        let js_accept_cookies = r#"
            let acceptBtn = document.querySelector('#data-protection-consent-dialog button[slot="primary-button"]');
            if (acceptBtn) {
                acceptBtn.click();
                return 'clicked';
            }
            
            let buttons = document.querySelectorAll('button');
            for (let btn of buttons) {
                if (btn.textContent.includes('Accept All')) {
                    btn.click();
                    return 'clicked';
                }
            }
            
            let dialog = document.querySelector('#data-protection-consent-dialog');
            if (dialog) {
                dialog.style.display = 'none';
            }
            
            let sheet = document.querySelector('#data-protection-consent-sheet');
            if (sheet) {
                sheet.setAttribute('open-state', 'closed');
                sheet.removeAttribute('open');
            }
            
            try {
                document.cookie = 'eu_cookie_v2=1; path=/; max-age=31536000';
            } catch(e) {}
            
            return 'dismissed';
        "#;
        
        let _ = self.driver.execute(js_accept_cookies, vec![]).await;
    }

    async fn login(&self) -> Result<()> {
        println!("\n[LOGIN] Logging into Reddit");
        self.driver.goto("https://www.reddit.com").await?;
        self.random_pause().await;
        self.human_scroll().await;
        self.random_pause().await;

        self.driver.goto("https://www.reddit.com/login").await?;
        sleep(Duration::from_secs(6)).await;

        self.handle_cookie_popup().await;
        let _ = self.save_screenshot("login_page.png").await;

        let username_field = self.driver
            .query(By::Id("login-username"))
            .or(By::Id("loginUsername"))
            .or(By::Css("input[name='username']"))
            .or(By::Css("input[type='text']"))
            .first()
            .await
            .context("Could not find username field")?;
        
        self.human_type(&username_field, &self.username).await?;
        self.random_pause().await;

        let password_field = self.driver
            .query(By::Id("login-password"))
            .or(By::Id("loginPassword"))
            .or(By::Css("input[name='password']"))
            .or(By::Css("input[type='password']"))
            .first()
            .await
            .context("Could not find password field")?;
        
        self.human_type(&password_field, &self.password).await?;
        self.random_pause().await;

        let login_result = self.driver
            .query(By::Css("button"))
            .with_text("Log In")
            .or(By::Css("button[type='submit']"))
            .first()
            .await;

        match login_result {
            Ok(button) => {
                sleep(Duration::from_millis(500)).await;
                button.click().await?;
            }
            Err(_) => {
                let submit_button = self.driver
                    .query(By::Css("button[type='submit']"))
                    .first()
                    .await
                    .context("Could not find login button")?;
                sleep(Duration::from_millis(500)).await;
                submit_button.click().await?;
            }
        }

        sleep(Duration::from_secs(6)).await;
        
        let current_url = self.driver.current_url().await?;
        if current_url.as_str().contains("/login") {
            let _ = self.save_screenshot("login_failed.png").await;
            return Err(anyhow::anyhow!("Login failed"));
        }
        
        println!("[SUCCESS] Logged in successfully\n");
        Ok(())
    }

    fn pick_random_subreddit(&self) -> SubredditConfig {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..self.config.subreddits.len());
        self.config.subreddits[index].clone()
    }

    async fn navigate_to_subreddit(&self, config: &SubredditConfig) -> Result<()> {
        let sort = config.sort.as_str();
        let subreddit = &config.name;
        
        let url = match sort {
            "top" => {
                let timeframe = config.timeframe.as_deref().unwrap_or("day");
                format!("https://www.reddit.com/r/{}/top?t={}", subreddit, timeframe)
            }
            "new" => format!("https://www.reddit.com/r/{}/new", subreddit),
            "rising" => format!("https://www.reddit.com/r/{}/rising", subreddit),
            "hot" | _ => format!("https://www.reddit.com/r/{}/hot", subreddit),
        };
        
        if self.verbose {
            println!("\n[NAV] Going to: {}", url);
        }
        self.driver.goto(&url).await?;
        sleep(Duration::from_secs(3)).await;
        
        self.human_scroll().await;
        self.random_pause().await;
        Ok(())
    }

    async fn get_post_info(&self, post_element: &WebElement) -> Result<(String, String, String, String)> {
        let title = post_element
            .query(By::Css("h3"))
            .or(By::Css("[slot='title']"))
            .or(By::Css("a[data-click-id='body']"))
            .first()
            .await
            .context("Could not find post title")?
            .text()
            .await?;

        let link_element = post_element
            .query(By::Css("a[data-click-id='body']"))
            .or(By::Css("a[href*='/comments/']"))
            .first()
            .await
            .context("Could not find post link")?;
            
        let link = link_element
            .attr("href")
            .await?
            .context("Post link has no href")?;

        let post_id = link.split("/comments/")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .unwrap_or("")
            .to_string();

        let post_body = post_element
            .query(By::Css("[slot='text-body']"))
            .or(By::Css(".md"))
            .or(By::Css("[data-click-id='text']"))
            .first()
            .await
            .ok()
            .and_then(|elem| {
                futures::executor::block_on(elem.text()).ok()
            })
            .unwrap_or_default();

        Ok((title, link, post_id, post_body))
    }

    async fn generate_comment(&self, post_title: &str, post_body: &str, subreddit: &str) -> Result<String> {
        if self.verbose {
            println!("[AI] Generating comment");
        }
        
        let body_preview = if post_body.len() > 200 {
            &post_body[..200]
        } else {
            post_body
        };

        let body_context = if !post_body.is_empty() {
            format!("\n\nPost content: {}", body_preview)
        } else {
            String::new()
        };

        let emoticon_instruction = if self.comment_count % 10 == 0 {
            "\n- You can use ONE simple emoticon if it feels natural: :P, :3, -_-, 0_0, O:, ):, (: etc"
        } else {
            ""
        };
        
        // Replace placeholders in the template
        let prompt = self.prompt_template
            .replace("{{SUBREDDIT}}", subreddit)
            .replace("{{TITLE}}", post_title)
            .replace("{{BODY_CONTEXT}}", &body_context)
            .replace("{{EMOTICON_INSTRUCTION}}", emoticon_instruction);

        let request = GenerationRequest::new(self.model.clone(), prompt);
        
        match self.ollama.generate(request).await {
            Ok(response) => {
                let mut comment = response.response.trim().to_string();
                
                comment = comment.replace('"', "");
                comment = comment.replace('"', "");
                comment = comment.replace('"', "");
                comment = comment.replace('\'', "");
                comment = comment.replace('\'', "");
                comment = comment.replace('\'', "");
                
                comment = comment.trim().to_string();
                
                if self.verbose {
                    println!("[AI] Generated: {}", comment);
                }
                Ok(comment)
            }
            Err(e) => {
                Err(anyhow::anyhow!("Ollama failed: {:?}", e))
            }
        }
    }

    async fn vote_on_comments(&self) -> Result<()> {
        if !self.upvote_enabled {
            if self.verbose {
                println!("[VOTING] Skipped (upvote feature disabled)");
            }
            return Ok(());
        }

        if self.verbose {
            println!("[VOTING] Checking comments to naturally vote");
        }
        sleep(Duration::from_secs(10)).await;

        let js_vote = r#"
            let voted = {upvoted: 0, downvoted: 0, skipped: 0};
            let myUsername = arguments[0].toLowerCase();

            let comments = document.querySelectorAll('.comment, .thing.comment, [data-testid="comment"]');
            
            for (let comment of comments) {
                try {
                    let authorElem = comment.querySelector('.author, a[href*="/user/"]');
                    let author = authorElem ? authorElem.textContent.trim().toLowerCase() : null;
                    if (!author || author === myUsername || author === 'automoderator') continue;

                    let rand = Math.random();
                    
                    if (rand < 0.083) {
                        let upvoteBtn = comment.querySelector('.arrow.up, button[aria-label="upvote"]');
                        if (upvoteBtn && !upvoteBtn.classList.contains('upmod')) {
                            upvoteBtn.click();
                            voted.upvoted++;
                            await new Promise(r => setTimeout(r, 150));
                        }
                    } else if (rand < 0.226) {
                        let downvoteBtn = comment.querySelector('.arrow.down, button[aria-label="downvote"]');
                        if (downvoteBtn && !downvoteBtn.classList.contains('downmod')) {
                            downvoteBtn.click();
                            voted.downvoted++;
                            await new Promise(r => setTimeout(r, 150));
                        }
                    } else {
                        voted.skipped++;
                    }
                } catch(e) {
                    console.log('Error voting:', e);
                }
            }
            return voted;
        "#;

        match self.driver.execute(js_vote, vec![serde_json::json!(self.username)]).await {
            Ok(result) => {
                if let Some(obj) = result.json().as_object() {
                    let upvoted = obj.get("upvoted").and_then(|v| v.as_u64()).unwrap_or(0);
                    let downvoted = obj.get("downvoted").and_then(|v| v.as_u64()).unwrap_or(0);
                    let skipped = obj.get("skipped").and_then(|v| v.as_u64()).unwrap_or(0);
                    
                    if (upvoted > 0 || downvoted > 0) && self.verbose {
                        println!("[VOTING] Up: {} | Down: {} | Skip: {}", upvoted, downvoted, skipped);
                    }
                }
            }
            Err(e) => {
                if self.verbose {
                    println!("[VOTING] Failed: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn post_comment(&mut self, post_url: &str, comment_text: &str, post_id: &str, subreddit: &str, title: &str) -> Result<()> {
        if self.verbose {
            println!("\n[POST] Posting comment");
        }
        
        let old_reddit_url = if post_url.starts_with("http://") || post_url.starts_with("https://") {
            post_url.replace("www.reddit.com", "old.reddit.com")
                    .replace("reddit.com", "old.reddit.com")
        } else if post_url.starts_with("/") {
            format!("https://old.reddit.com{}", post_url)
        } else {
            format!("https://old.reddit.com/{}", post_url)
        };
        
        let mut retries = 3;
        while retries > 0 {
            match self.driver.goto(&old_reddit_url).await {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(anyhow::anyhow!("Navigation failed: {}", e));
                    }
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }
        
        sleep(Duration::from_secs(3)).await;
        
        let current_url = self.driver.current_url().await?;
        if !current_url.as_str().contains("old.reddit.com") {
            return Err(anyhow::anyhow!("Not on old Reddit"));
        }
        
        self.handle_cookie_popup().await;
        self.human_scroll().await;
        self.random_pause().await;

        let comment_box = self.driver
            .query(By::Css("textarea[name='text']"))
            .or(By::Css("textarea.usertext-edit"))
            .or(By::Css("div.usertext-edit textarea"))
            .first()
            .await
            .context("Could not find comment textarea")?;
        
        let _ = self.driver.execute(
            "arguments[0].scrollIntoView({block: 'center'}); arguments[0].focus();",
            vec![comment_box.to_json()?]
        ).await;
        self.random_pause().await;

        self.human_type(&comment_box, comment_text).await?;
        self.random_pause().await;

        let _ = self.save_screenshot("before_submit.png").await;

        let submit_button = self.driver
            .query(By::Css("button.save"))
            .or(By::Css("button[type='submit'].save"))
            .first()
            .await
            .context("Could not find save button")?;

        let _ = self.driver.execute(
            "arguments[0].scrollIntoView({block: 'center'});",
            vec![submit_button.to_json()?]
        ).await;
        self.random_pause().await;
        
        submit_button.click().await?;
        sleep(Duration::from_secs(4)).await;

        let _ = self.vote_on_comments().await;

        self.commented_posts.insert(post_id.to_string());
        self.save_posted(post_id, subreddit, title)?;

        println!("[SUCCESS] Comment posted\n");
        Ok(())
    }

    async fn run_bot(&mut self) -> Result<()> {
        println!("{}", "=".repeat(64));
        println!("Reddit Bot Starting - Continuous Mode");
        println!("{}\n", "=".repeat(64));
        
        self.login().await?;
        
        let mut comments_posted = 0;
        let mut consecutive_failures = 0;
        let mut rng = rand::thread_rng();

        loop {
            if consecutive_failures >= 10 {
                println!("\n[WARNING] 10 consecutive failures. Waiting 5 minutes before retry");
                sleep(Duration::from_secs(300)).await;
                consecutive_failures = 0;
                
                match self.login().await {
                    Ok(_) => println!("[SUCCESS] Re-logged in successfully"),
                    Err(e) => {
                        println!("[ERROR] Re-login failed: {}. Waiting another 5 minutes", e);
                        sleep(Duration::from_secs(300)).await;
                        continue;
                    }
                }
            }

            let subreddit_config = self.pick_random_subreddit();

            println!("{}", "=".repeat(64));
            println!("Random pick: r/{} [{}]", subreddit_config.name, subreddit_config.sort);
            
            let bar_width = 50;
            let filled = ((comments_posted % 100) as f32 / 100.0 * bar_width as f32) as usize;
            let empty = "░".repeat(bar_width - filled);
            let bar = format!("{}{}", "█".repeat(filled), empty);
            println!("[{}] Total comments: {}", bar, comments_posted);
            println!("{}\n", "=".repeat(64));

            match self.navigate_to_subreddit(&subreddit_config).await {
                Ok(_) => {},
                Err(e) => {
                    println!("[ERROR] Navigation failed: {}", e);
                    consecutive_failures += 1;
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            }

            let posts = match self.driver
                .query(By::Css("shreddit-post"))
                .or(By::Css("div[data-testid='post-container']"))
                .all_from_selector()
                .await {
                    Ok(p) => p,
                    Err(e) => {
                        println!("[ERROR] No posts found: {}", e);
                        consecutive_failures += 1;
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

            if posts.is_empty() {
                println!("[WARNING] No posts, trying another subreddit");
                continue;
            }

            let max_posts_to_check = posts.len().min(20);
            
            let mut found_post = false;
            for _ in 0..max_posts_to_check {
                let post_index = rng.gen_range(0..max_posts_to_check);
                let post = &posts[post_index];

                match self.get_post_info(post).await {
                    Ok((title, link, post_id, post_body)) => {
                        if self.commented_posts.contains(&post_id) {
                            continue;
                        }

                        println!("\n[FOUND] \"{}\"", title);
                        if !post_body.is_empty() && self.verbose {
                            let preview = if post_body.len() > 100 {
                                format!("{}...", &post_body[..100])
                            } else {
                                post_body.clone()
                            };
                            println!("[BODY] {}", preview);
                        }
                        found_post = true;
                        
                        let comment = match self.generate_comment(&title, &post_body, &subreddit_config.name).await {
                            Ok(c) => c,
                            Err(e) => {
                                println!("[ERROR] AI failed: {}", e);
                                consecutive_failures += 1;
                                sleep(Duration::from_secs(5)).await;
                                break;
                            }
                        };
                        
                        match self.post_comment(&link, &comment, &post_id, &subreddit_config.name, &title).await {
                            Ok(_) => {
                                comments_posted += 1;
                                self.comment_count += 1;
                                consecutive_failures = 0;
                                println!("[SUCCESS] Total comments posted: {}", comments_posted);
                                
                                let wait_time = rng.gen_range(60..=600);
                                println!("[WAIT] Chilling for {} seconds before next comment\n", wait_time);
                                sleep(Duration::from_secs(wait_time)).await;
                                break;
                            }
                            Err(e) => {
                                println!("[ERROR] Failed to post: {}", e);
                                consecutive_failures += 1;
                                sleep(Duration::from_secs(5)).await;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        if self.verbose {
                            println!("[SKIP] Error: {}", e);
                        }
                        continue;
                    }
                }
            }

            if !found_post {
                println!("[WARNING] No uncommented posts, picking another random subreddit");
                sleep(Duration::from_secs(3)).await;
            }
        }
    }

    async fn quit(self) -> Result<()> {
        self.driver.quit().await?;
        Ok(())
    }
}

fn check_first_run_acknowledgment() -> Result<()> {
    const ACK_FILE: &str = ".reddit_bot_ack";
    
    if fs::metadata(ACK_FILE).is_ok() {
        return Ok(());
    }

    println!("\n{}", "=".repeat(64));
    println!("TERMS ACKNOWLEDGMENT");
    println!("{}", "=".repeat(64));
    println!("\nBefore using this software, you must acknowledge the following:\n");
    println!("This software may violate Reddit's Terms of Service.");
    println!("You are solely responsible for any consequences of using this tool.");
    println!("The creator of this software is not responsible for your actions.\n");
    println!("{}", "=".repeat(64));
    println!("\nTo continue, type EXACTLY:\n");
    println!("I disagree with Reddit's TOS. I don't hold the creator of this software responsible for any of my actions. Solely I and I alone am responsible for any damages.");
    println!("\n{}", "=".repeat(64));
    print!("\nResponse: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let input = input.trim();

    let expected = "I disagree with Reddit's TOS. I don't hold the creator of this software responsible for any of my actions. Solely I and I alone am responsible for any damages.";

    if input != expected {
        println!("\n[ERROR] Acknowledgment text does not match. Quitting.");
        std::process::exit(1);
    }

    fs::write(ACK_FILE, "acknowledged")?;
    println!("\n[SUCCESS] Acknowledgment saved. Can\'t hold me responsible now! You will not be asked again.\n");
    
    Ok(())
}

fn check_upvote_acknowledgment(upvote_enabled: bool) -> Result<()> {
    if !upvote_enabled {
        return Ok(());
    }

    const UPVOTE_ACK_FILE: &str = ".reddit_bot_upvote_ack";
    
    if fs::metadata(UPVOTE_ACK_FILE).is_ok() {
        return Ok(());
    }

    println!("\n{}", "=".repeat(64));
    println!("UPVOTE FEATURE IS BROKEN");
    println!("{}", "=".repeat(64));
    println!("\nYou have enabled the --upvote/-u flag.\n");
    println!("Using the upvote feature significantly increases the");
    println!("risk of detection and catching a ban.\n");
    println!("Reddit's anti-bot systems can detect automated voting patterns.");
    println!("{}", "=".repeat(64));
    println!("\nTo continue with upvoting enabled, type EXACTLY:\n");
    println!("I recognize that using the upvote feature will get me banned.");
    println!("\n{}", "=".repeat(64));
    print!("\nYour response: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let input = input.trim();

    let expected = "I recognize that using the upvote feature will get me banned.";

    if input != expected {
        println!("\n[ERROR] Acknowledgment text does not match. Exiting.");
        std::process::exit(1);
    }

    fs::write(UPVOTE_ACK_FILE, "acknowledged")?;
    println!("\n[SUCCESS] Upvote acknowledgment recorded.\n");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    check_first_run_acknowledgment()?;
    check_upvote_acknowledgment(args.upvote)?;

    println!("\n{}", "=".repeat(64));
    println!("   Reddit Comment Bot");
    println!("{}", "=".repeat(64));
    println!("\nModel: {}", args.model);
    println!("Mode: {}", if args.headless { "Headless" } else { "Visible browser" });
    println!("Upvote: {}\n", if args.upvote { "ENABLED (HIGH RISK)" } else { "Disabled" });

    let username = std::env::var("REDDIT_USERNAME")
        .expect("Set REDDIT_USERNAME environment variable");
    let password = std::env::var("REDDIT_PASSWORD")
        .expect("Set REDDIT_PASSWORD environment variable");

    let mut bot = RedditBot::new(username, password, &args).await?;
    
    println!("Bot will run continuously. Press Ctrl+C to force quit.\n");
    
    match bot.run_bot().await {
        Ok(_) => println!("\nBot finished"),
        Err(e) => eprintln!("\nBot error: {}", e),
    }
    
    bot.quit().await?;
    Ok(())
}