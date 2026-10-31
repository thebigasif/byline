// The blog's front page: every post in the site's main language, newest
// first, each linking to its page.
import Link from "next/link";
import { getPosts, getSettings, routeOf, siteFrom } from "../lib/khalid.js";

export const revalidate = 300;

export default async function Home() {
  const site = siteFrom(process.env);
  const settings = await getSettings(site);
  const posts = (await getPosts(site)).filter(
    (post) => post.language === settings.default_language,
  );
  return (
    <>
      <title>{settings.title}</title>
      <h1>{settings.title}</h1>
      <ul>
        {posts.map((post) => (
          <li key={post.id}>
            <Link href={`/${routeOf(post, "/blog/")}/`}>{post.title}</Link>
            {post.excerpt && <p>{post.excerpt}</p>}
          </li>
        ))}
      </ul>
    </>
  );
}
