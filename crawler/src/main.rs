#![feature(try_blocks)]

use anyhow::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    use sqlx::Connection;

    tracing_subscriber::fmt::init();

    #[derive(serde::Deserialize)]
    struct Settings {
        /// https://console.anthropic.com/settings/keys
        anthropic_api_key: String,

        /// i.e. postgres://postgres@localhost/my_database
        database_url: String,
    }

    let Settings {
        ref anthropic_api_key,

        ref database_url,
    } = config::Config::builder()
        //
        .add_source(config::Environment::with_prefix("CRAWLER"))
        //
        .build()?
        //
        .try_deserialize()
        //
        .context("failed to deserialize settings")?;

    let mut database_connection = sqlx::PgConnection::connect(database_url)
        //
        .await
        //
        .context("failed to connect to the database")?;

    loop {
        tracing::info!("Crawling...");

        let result: Result<()> = try {
            use rss::*;

            let channel: Result<Channel> = try {
                let bytes = reqwest::get("https://www.cbc.ca/webfeed/rss/rss-politics")
                    //
                    .await?
                    //
                    .bytes()
                    //
                    .await?;

                let channel = Channel::read_from(&bytes[..])?;

                channel
            };

            let Channel { items, .. } = channel.context("failed to read RSS channel")?;

            for item in items
                //
                .into_iter()
                //
                .rev()
            {
                let result: Result<()> = try {
                    let Some(ref title) = item.title else {
                        continue;
                    };

                    let Some(ref link) = item.link else {
                        continue;
                    };

                    tracing::info!(%title, %link, "Processing item");

                    //
                    let slug = slug::slugify(title);

                    //
                    let exists: Result<bool> = try {
                        let row: (bool,) =
                            sqlx::query_as("select exists(select 1 from articles where slug = $1)")
                                //
                                .bind(&slug)
                                //
                                .fetch_one(&mut database_connection)
                                //
                                .await?;

                        let exists = row.0;

                        exists
                    };

                    if exists.context("failed to check if article exists in the database")? {
                        tracing::info!(%slug, "Article already exists, skipping.");

                        continue;
                    }

                    //
                    let data: Result<(String, String, String)> = try {
                        use select::{document::Document, predicate::*};

                        let bytes = reqwest::get(link)
                            //
                            .await?
                            //
                            .bytes()
                            //
                            .await?;

                        let document = Document::from_read(&bytes[..])?;

                        //
                        let story = document
                            //
                            .find(Name("div").and(Class("story")))
                            //
                            .next()
                            //
                            .map(|node| node.text())
                            //
                            .ok_or_else(|| anyhow!("failed to find plot in the document"))?;

                        let images = document
                            //
                            .find(Name("img").and(Attr("data-cy", "image-img")))
                            //
                            .map(|node| node.html())
                            //
                            .collect::<String>();

                        let thumbnail_url = document
                            //
                            .find(Name("img").and(Attr("data-cy", "leadmedia-story-img")))
                            //
                            .next()
                            //
                            .and_then(|node| node.attr("src"))
                            //
                            .map(|src| src.to_string())
                            //
                            .ok_or_else(|| anyhow!("failed to find thumbnail in the document"))?;

                        (story, images, thumbnail_url)
                    };

                    let (plot, images, thumbnail_url) = data
                        //
                        .context("failed to parse article data")?;

                    let data: Result<(String, String)> = try {
                        let prompt = format!(
                            r#"
                                <plot>
                                  {plot}
                                </plot>
                                                               
                                <images>
                                  {images}
                                </images>

                                You are to embody the spirit and philosophy of Friedrich Nietzsche while writing a news article based on a given plot. Your task is to infuse the article with Nietzschean concepts and commentary, particularly focusing on the ideas of the Superman, nihilism, the land of the sleepers, and the last man.

                                The <plot> tag contains the plot for your news article.

                                The <images> tag contains HTML <img> tags that you must insert into appropriate places within the article.

                                Write the news article of 1500-2000 words from Nietzsche's perspective, adhering to the following guidelines:

                                1. Write in a style that mimics Nietzsche's aphoristic, poetic, and often confrontational tone. Use metaphors and allegories liberally.

                                2. Incorporate commentary as if you were the Superman (Übermensch) observing the events and the people involved. This commentary should be interspersed throughout the article, set apart in <blockquote>.

                                3. Describe the setting or society in which the plot takes place as "the land of the sleepers." How are the masses unaware or complacent in the face of the events unfolding?

                                4. Identify and criticize elements of "the last man" in the characters or society depicted in the plot. How do they embody complacency, comfort-seeking, or a lack of aspiration?

                                5. Conclude the article with a powerful statement.

                                6. Do not the words nihilism or Nietzsche directly.

                                7. Use archaic English.

                                Use <p> tags for paragraphs. The Superman's interjections should be in <blockquote>.

                                Remember, your goal is not just to report the events, but to use them as a canvas for exploring and expressing Nietzschean philosophy. Be bold, provocative, and unapologetic in your writing, as Nietzsche himself would have been.

                                Output following this example format:

                                <article><title>title</title><content>content</content></article>

                                Strictly follow this format. Do not insert line breaks.
                            "#
                        );

                        tracing::info!("Sending a request to Anthropic API");

                        let response: serde_json::Value = reqwest::Client::new()
                            //
                            .post("https://api.anthropic.com/v1/messages")
                            //
                            .header("x-api-key", anthropic_api_key)
                            //
                            .header("anthropic-version", "2023-06-01")
                            //
                            .header("content-type", "application/json")
                            //
                            .json(
                                //
                                &serde_json::json!(
                                    {
                                        "model": "claude-3-5-sonnet-latest",

                                        "max_tokens": 8192,

                                        "system": "You are Friedrich Nietzsche.",

                                        "temperature": 1.0,

                                        "messages": [
                                            { "role": "user", "content": prompt },

                                            { "role": "assistant", "content": "<article><title>" },
                                        ]
                                    }
                                ),
                            )
                            //
                            .send()
                            //
                            .await?
                            //
                            .json()
                            //
                            .await?;

                        let text = response
                            //
                            .get("content")
                            //
                            .and_then(|content| content.get(0))
                            //
                            .and_then(|item| item.get("text"))
                            //
                            .and_then(|text| {
                                text
                                    //
                                    .as_str()
                                    //
                                    .map(|text| text.trim_matches('\"').to_string())
                            })
                            //
                            .ok_or_else(|| anyhow!("failed to find text in the api response"))?;

                        fn substring_between(s: &str, start: &str, end: &str) -> Option<String> {
                            if let Some(start_index) = s.find(start) {
                                let start_index = start_index + start.len();

                                if let Some(end_index) = s[start_index..].find(end) {
                                    let substring = s[start_index..start_index + end_index]
                                        //
                                        .to_string();

                                    return Some(substring);
                                }
                            }

                            None
                        }

                        let title = substring_between(&text, "", "</title>")
                            //
                            .ok_or_else(|| anyhow!("failed to find article title in the text"))?;

                        let content = substring_between(&text, "<content>", "</content>")
                            //
                            .ok_or_else(|| anyhow!("failed to find article content in the text"))?;

                        (title, content)
                    };

                    let (title, content) = data.context("failed to generate article content")?;

                    let result: Result<()> = try {
                        sqlx::query("insert into articles (slug, thumbnail_url, title, content) values ($1, $2, $3, $4)")
                            //
                            .bind(&slug)
                            //
                            .bind(&thumbnail_url)
                            //
                            .bind(&title)
                            //
                            .bind(&content)
                            //
                            .execute(&mut database_connection)
                            //
                            .await?;

                        tracing::info!(%slug, "Article saved successsfully");
                    };

                    result.context("failed to save article to the database")?;
                };

                if let Err(error) = result {
                    tracing::error!(?error, "failed to processs an item");
                }
            }
        };

        if let Err(error) = result {
            tracing::error!(?error, "failed to crawl");
        }

        tokio::time::sleep(
            //
            tokio::time::Duration::from_secs(60),
        )
        //
        .await;
    }
}
