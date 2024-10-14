create table articles(
  slug text not null primary key,
  title text not null,
  thumbnail_url text not null,
  content text not null,
  time_created timestamp not null default now()
);
