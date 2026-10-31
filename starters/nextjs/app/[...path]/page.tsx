// One page per post, at the address Khalid gives it, such as
// /blog/what-a-sitemap-does/ or /blog/de/was-eine-sitemap-leistet/. The
// head is the one Khalid's theme writes, so meta tags and schema match.
import { notFound } from "next/navigation";
import { getPost, getPosts, routeOf, siteFrom } from "../../lib/khalid.js";
import { KhalidHead } from "../khalid-head";

export const revalidate = 300;

type Params = { path: string[] };

export async function generateStaticParams(): Promise<Params[]> {
  const posts = await getPosts(siteFrom(process.env));
  return posts.map((post) => ({ path: routeOf(post, "/blog/").split("/") }));
}

export default async function PostPage({ params }: { params: Promise<Params> }) {
  const { path } = await params;
  const site = siteFrom(process.env);
  const route = path.map(decodeURIComponent).join("/");
  const listed = (await getPosts(site)).find((post) => routeOf(post, "/blog/") === route);
  const post = listed && (await getPost(site, listed.id));
  if (!post) {
    notFound();
  }
  return (
    <article lang={post.language}>
      {post.head ? <KhalidHead head={post.head} /> : <title>{post.title}</title>}
      <h1>{post.title}</h1>
      <p>
        <time dateTime={post.published_at}>{post.published_at.slice(0, 10)}</time>
      </p>
      <div dangerouslySetInnerHTML={{ __html: post.html ?? "" }} />
    </article>
  );
}
