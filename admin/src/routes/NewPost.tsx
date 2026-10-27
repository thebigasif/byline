import { StartPost } from "../components/StartPost";
import { en } from "../strings/en";

/** The screen behind "Write a new post": the start form on its own. */
export function NewPost() {
  return (
    <>
      <h1>{en.posts.write}</h1>
      <StartPost />
    </>
  );
}
