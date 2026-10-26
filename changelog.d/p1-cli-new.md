Added
`khalid --site <folder> new "Title"` writes a post to `content/<language>/posts/<date>-<slug>.md` in the site's default language, with its title, slug and today's date in the front matter. The slug is made from the title in that language, so "Über Größe" becomes `ueber-groesse` on a German site. It never replaces a post, and the file is written atomically (BL-RUN-9, KH-OPS-3).
