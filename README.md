# Redditard, Automated Reddit Commenting Bot
This program uses Selenium/Thirtyfour WebDriver and Ollama to automatically browse Reddit, generate human-esque comments using AI, and post them to various subreddits with 'natural' behaviour patterns.

Your account will eventually get flagged and your IP will be banned, I NEITHER CARE NOR AM RESPONSIBLE FOR HOW YOU USE THIS PROGRAM. ALL ON YOU!

## Features
- Logs into Reddit and navigates subreddits automatically
- Generates contextually relevant comments using local LLM (Ollama)
- Mimics human behavior (random scrolling, typing delays, pauses)
- Supports multiple subreddits with different sort modes
- Customizable AI prompt via configuration file
- Tracks commented posts to avoid duplicates
- Optional voting on other comments (high risk feature, you will get flagged quickly)
- Headless or visible browser modes

## Requirements
- Rust & Cargo
- ChromeDriver **running on port 9517**
- Ollama installed **and running**
- Reddit account credentials (gross)

## Compile
```bash
cargo build --release
```

## Configuration Files

### subreddits.toml
Define which subreddits to target and how to sort posts:
```toml
[[subreddits]]
name = "unixporn"
sort = "hot"

[[subreddits]]
name = "programming"
sort = "top"
timeframe = "day"

[[subreddits]]
name = "askreddit"
sort = "rising"
```

### prompt.toml (Optional)
Customize the AI prompt used to generate comments. If not present, uses default prompt.
```toml
custom_prompt = """
You're browsing r/{{SUBREDDIT}} and just saw: {{TITLE}}{{BODY_CONTEXT}}

Write a quick 1-2 sentence reaction.
Just write the comment, nothing else:
"""
```

**Available placeholders:**
- `{{SUBREDDIT}}` - Subreddit name
- `{{TITLE}}` - Post title
- `{{BODY_CONTEXT}}` - Post content preview

## Environment Variables
Set your Reddit credentials:
```bash
export REDDIT_USERNAME="username"
export REDDIT_PASSWORD="password"
```
This isn't safe.

## Flags
| Option | Description | Default |
|--------|-------------|---------|
| `--model`, `-m` | Ollama model name | `deepseek-r1:latest` |
| `--headless`, `-H` | Run browser in headless mode | `false` |
| `--verbose`, `-v` | Print detailed debug information | `false` |
| `--min-interval`, `-i` | Minimum seconds between comments | `60` |
| `--max-interval`, `-x` | Maximum seconds between comments | `600` |
| `--upvote`, `-u` | Enable voting on comments (**HIGH RISK**) | `false` |

## Output Files
This program will output a lot of pointless files.
- `posted.txt` - Log of all commented posts with timestamps
- `login_page.png` - Screenshot of login page (if verbose)
- `before_submit.png` - Screenshot before submitting comment (if verbose)
- `.reddit_bot_ack` - First run acknowledgment flag
- `.reddit_bot_upvote_ack` - Upvote feature acknowledgment flag

## Behaviour
- Randomly selects subreddit from config
- Checks up to 20 posts per subreddit visit
- Skips already commented posts
- Waits 60 to 600 seconds between comments
- Uses human-like typing (20-80ms per character)
- Random scrolling and pauses
- Rotates user agents for anonymity
- Optional comment voting (8.3% upvote, 14.3% downvote, 77.4% skip)

## Troubleshooting
- **ChromeDriver errors**: Ensure ChromeDriver is running on port 9517
- **Ollama fails**: Verify Ollama service is running and model used is downloaded
- **No posts found**: Check subreddit names in `subreddits.toml`, else blame reddit updating
This program isn't time proof. Reddit slightly changing its UI will break it. It's open-source so if you like it you can fix it yourself :)

## Disclaimer
- This project violates Reddit’s Terms of Service.
- Accounts and IPs used with this software will likely be flagged or banned.
- You are solely responsible for how you use this.

Your account will eventually get flagged and your IP will be banned, I NEITHER CARE NOR AM RESPONSIBLE FOR HOW YOU USE THIS PROGRAM. ALL ON YOU!
