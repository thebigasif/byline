# Theme sync on ten real sites, 2026-10-27

What this is: the hand check KH-WEB-8 asks for ("eight of ten real sites look native without edits"), run for audit brief G21 with `cargo run -p khalid -- --site <folder> theme sync <url>` from the theme track's server, at the tip of the G21 stack (`task/p1-theme-ten-sites`).

What it is not: nobody looked at the previews in a browser. The server has no browser, so "looks native" below is judged from the tokens against what each site is known to show, not seen. The founder should open the ten previews (or run the same ten syncs) and correct the last column.

## Results

| Site | Background | Text | Link | Body font | Logo, icon | What sync said | Judged from the tokens |
|---|---|---|---|---|---|---|---|
| rust-lang.org | `#ffffff` | `#1f2328` (default) | `#0b57d0` (default) | Fira Sans | both | nothing | likely native; link is the default blue |
| python.org | `#ffffff` | `#444444` | `#3776ab` | SourceSansProRegular | both | quiet text and the yellow accent darkened for contrast | likely native |
| djangoproject.com | `#f8f8f8` | `#0c3c26` | `#177c56` | Roboto | icon only | link, accent and quiet text darkened for contrast | likely native, a darker green link |
| getbootstrap.com | `#712cf9` | `#dddddd` | `#ccdffc` | Geist | icon only | text, link and quiet text changed for contrast | **wrong**: the violet of the hero became the page background |
| sqlite.org | `#ffffff` | `#044a64` | `#044a64` | Verdana | logo only | quiet text darkened | likely native |
| debian.org | `#fcfcfc` | `#000000` | `#0b57d0` (default) | system | both | nothing | likely native; link is the default blue |
| nodejs.org | `#ffffff` | `#0d121c` | `#0b57d0` (default) | system | icon only | the pale green accent darkened | likely native; link is the default blue |
| mozilla.org | `#ffffff` | `#000000` | `#000000` | Mozilla Text | both | nothing | likely native |
| stripe.com | none | none | none | none | none | "found no stylesheet": its CSS is on another host | **failed**: nothing written |
| gnome.org | `#eeeeee` | `#000000` | `#1a5fb4` | InterVariable | icon only | InterVariable is on static.gnome.org, licence unknown | unsure; the page may be white |

Count: seven likely native, one unsure, two not. That is below the eight of ten the PRD asks, before anyone has looked.

## Faults found and fixed in the same stack

- The bzip2 manual and the istanbul report (`crates/khalid-theme/fixtures/real/`) showed frequency taking the grey of a code box or a table as the page background. Fixed in #644 and in this part.
- Python's icon font Pythonicon was named as a font the blog may not serve, with an empty server name: it is written inside the CSS as a `data:` address, and it is not a text font. Fixed here: only the first family of the body and heading lists can be held, and a `data:` font counts as the site's own.
- GNOME's InterVariable was said to have a licence that "does not let the blog serve it". Inter is openly licensed; what is true is that Khalid cannot tell. The sentence now says that for every server but Adobe Fonts.

## Faults found and left open

- **Stylesheets on a CDN host** (stripe.com, `b.stripecdn.com`). ADR 0015 reads same-site stylesheets only. Reading a stylesheet from the host the page itself links, through `Outbound` with the same caps, would fix Stripe and many sites behind a CDN. It needs an ADR change, so it is a follow-up.
- **A hero colour taken as the page** (getbootstrap.com). Not diagnosed: the stylesheet was not saved, and the server cannot show the page. The next step is to save the CSS as a fixture and find which rule wins.
- **Links left at the default blue** on rust-lang.org, debian.org and nodejs.org, which style links through classes or nested selectors the link pass does not read.
