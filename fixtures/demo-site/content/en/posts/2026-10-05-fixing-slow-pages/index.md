---
title: Fixing slow pages, starting with the biggest win
slug: fixing-slow-pages
date: 2026-10-05
author: lena-vogt
category: technical-seo
description: Slow pages lose readers before they read a word. Where the time usually goes, and what to fix first.
translation: page-speed
---

Readers leave pages that load slowly, and search engines notice. The good news: most slow pages are slow for the same few reasons.

## Measure first

Run the page through a speed test on a phone setting, not a desktop one. Write down the largest contentful paint, which is the time until the main content is visible. Under one second is great. Over two and a half seconds needs work.

## The usual causes, biggest first

:::figure{src="page-weight.png" alt="Bar chart of a slow page's bytes: images take the most, then scripts, fonts, styles and HTML"}
Where the bytes of a typical slow page go.
:::

1. **Images that are too large.** A photo saved at 4,000 pixels wide for a slot 800 pixels wide wastes most of its bytes. Serve the size the page needs, in a modern format.
2. **Scripts the reader does not need.** Every chat widget and tracking tag delays the page. Remove the ones nobody reads the data from.
3. **Fonts that block the text.** Show the text in a system font first, and swap in the web font when it arrives.
4. **Layout that jumps.** Give every image a width and height, so the text does not move while the page loads.

Fix the first cause on your slowest page, measure again, and move on. Faster pages also get crawled more often, which helps [your sitemap](/blog/what-a-sitemap-does/) do its job.
