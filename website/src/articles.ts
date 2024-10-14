import postgres from "postgres";

export async function fetchArticles(env: any): Promise<{ slug: string; title: string; thumbnailUrl: string }[]> {
  const sql = postgres({
    host: env.DB_HOST,
    port: parseInt(env.DB_PORT, 10),
    username: env.DB_USER,
    password: env.DB_PASSWORD,
    database: env.DB_DATABASE,
  });

  try {
    const res = await sql`
      select slug, title, thumbnail_url
      from articles
      order by time_created desc
      limit 24
    `;

    return res.map(row => ({
      slug: row.slug,
      title: row.title,
      thumbnailUrl: row.thumbnail_url,
    }));
  } catch (err) {
    console.error("Error fetching articles:", err);
    throw err;
  }
}

export async function fetchArticleBySlug(env: any, slug: string): Promise<{ title: string; content: string }> {
  const sql = postgres({
    host: env.DB_HOST,
    port: parseInt(env.DB_PORT, 10),
    username: env.DB_USER,
    password: env.DB_PASSWORD,
    database: env.DB_DATABASE,
  });

  try {
    const res = await sql`
      select title, content
      from articles
      where slug = ${slug}
    `;

    if (res.length === 0) {
      throw new Error(`Article with slug "${slug}" not found`);
    }

    const row = res[0];

    return {
      title: row.title,
      content: row.content,
    };
  } catch (err) {
    console.error("Error fetching article by slug:", err);
    throw err;
  }
}
