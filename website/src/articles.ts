export async function fetchArticleCount(
  //
  sql: any,
): Promise<number> {
  const res = await sql`select count(*) as count from articles`;

  return res[0].count;
}

export async function fetchArticles(
  //
  sql: any,
  //
  page: number,
  //
  pageSize: number,
): Promise<{ slug: string; title: string; thumbnailUrl: string }[]> {
  const res = await sql`
    select slug, title, thumbnail_url
    from articles
    order by time_created desc
    offset ${page * pageSize} limit ${pageSize}
  `;

  return res.map(row => ({
    slug: row.slug,
    title: row.title,
    thumbnailUrl: row.thumbnail_url,
  }));
}

export async function fetchArticleBySlug(
  //
  sql: any,
  //
  slug: string,
): Promise<{ title: string; content: string }> {
  const res = await sql`
    select title, content
    from articles
    where slug = ${slug}
  `;

  if (res.length === 0) {
    throw new Error(`Article with slug "${slug}" not found`);
  }

  const row = res[0];

  return { title: row.title, content: row.content };
}
