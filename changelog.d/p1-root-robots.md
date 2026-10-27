Added
For a blog in a folder, such as `/blog/`, `khalid build` writes the host's `robots.txt`, and any verification files, into `_root/` in the output folder and says to copy them to the root of the main site, where search engines look. `khalid serve` answers `/robots.txt` at the root of its host too. A blog at the root of its host is unchanged (KH-SEO-5, audit G3).
