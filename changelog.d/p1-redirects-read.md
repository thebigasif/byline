Added
Redirect lists are read from CSV with the columns `from,to,status,match`, the same file every importer writes: 301, 302, 307, 410 and 451, exact paths or regular expressions. A bad row is refused with its line number, and the whole list with it (KH-SEO-8).
