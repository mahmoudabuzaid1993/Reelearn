# Logged-in web app teardown (app.deepstash.com) — observed 2026-10-29

Viewport ~731px wide (narrow, mobile-like layout). User account signed in; account state: 7 unread notifications, "Write in Studio" button present.

## Home (/)
- Top bar: logo, search "Ideas, topics & more...", "Write in Studio", bell (7), avatar (Settings).
- Feed tabs (pill chips): Daily Picks | Recommended | Following | Collections.
- DAILY PICKS: 3 items — "The Courage to Be Disliked" (curator Dhaval Soni, 11 ideas, 19.5K reads), "Valuable Humans in Transit" (curator Deepstash, 10 ideas, 64.7K reads), "Japanese Philosophy" (curator Dana G, 14 ideas, 649K reads).
- RECOMMENDED: repeats Japanese Philosophy, then Hooked (Fazal N., 10 ideas, 5.75K reads, with a long curator blurb), Educated (Deepstash, 10 ideas, 75.5K reads). Overlap between Daily Picks and Recommended = weak dedup.
- FOLLOWING: empty state — "Discover ideas from other stashers... Featuring @yz_anroid, @jamesthecooper and 8 others" + "See suggested stashers" + "Follow All".
- COLLECTIONS: 6 emoji-titled collections (Back to School Basics for Parents 78 ideas, Productivity Hacks 109, Beat Procrastination 99, Boost Your Emotional Intelligence 40, Managing Time Like a Pro 88, Ask for a Raise 64) + "Explore all ideas".
- Bottom promo: "51% OFF Special Sale Lifetime Plan — Save unlimited ideas / Listen to ideas / Start unlimited journeys — Claim Lifetime Offer — Only ~~$407.15~~ $199.99. Pay once." (anchor price $407.15 looks invented for discount framing)
- Footer: Content (Ideas, Collections, Stories, Explore) / Product (Pricing, Businesses) / Resources (Terms, Privacy, Press Kit, Sitemap) / Company (About, Contact).
- Card unit on home = a "stash"/summary: curator avatar+name, cover, title, author, "N ideas · N reads".

## Logged-out landing (seen before sign-in)
- Hero "BECOME THE SMARTEST IN THE ROOM", "200,000+ powerful ideas", goal picker (Increase Productivity / Win at Work / Build Better Habits / Earn More Money) → Continue.
- Comparison table without/with Deepstash/with Pro: ideas access, mobile app access, unlimited saving, unlimited history, unlimited listening, downloads/offline, personalized microlearning.
- Social proof: "4.8 / 14,500+ reviews App Store; 4.6 / 92,000+ Google Play" (stale vs 85K iOS / ~198K Android now).
- 22 topic chips.

## Book page (/article/215915/the-courage-to-be-disliked)
- Breadcrumb Home / Books / title. Heading "Dhaval Soni's Key Ideas from The Courage to Be Disliked" — i.e. the page is one curator's stash, not an official summary.
- Topic tags, "11 ideas · 19.5K reads", action row: share, headphones (listen), bookmark (save), heart (82), comment (2), "...".
- Ideas are stacked vertically as cards: title + 1–3 short paragraphs, each with its own like count (~290–336) and reads (2.65K → 1.24K declining down the page: classic drop-off curve ~53% from first to last idea).
- Final card is a quote card attributed "ICHIRO KASHIMI" (typo for Kishimi) — UGC quality issue.
- Sidebar: "IDEAS CURATED BY Dhaval Soni @dhavalsoni5 FOLLOW".
- "Different Perspectives Curated by Others": book summary hub page + 3 other user stashes (Deepstash official 7 ideas, Giovanna Renee Tan 3, Nicholas York 10) — multiple competing summaries per book.
- "Discover Key Ideas from Books on Similar Topics" — includes a list25.com movie-quotes article (weak relevance) and Civilization and Its Discontents twice (dup).
- Direct URL navigation triggered a Cloudflare bot challenge; in-app click navigation did not.
- /pricing → 404 (footer link).

## Idea card interaction (web)
- Clicking an idea card on a stash page does nothing — cards are inline only; no dedicated idea view/swipe mode on web. Each card has: bookmark icon + like count on left, "N reads" + kebab on right.
- Layout bug at ~731px: the second idea card renders overlapping the "Different Perspectives Curated by Others" heading (z-index/sticky issue).

## Book hub page (/book/1736/the-courage-to-be-disliked) — title tag "...Summary 2024"
- "Your one stop book summary, audiobook and book review spot for: <title>" — SEO-style copy. Publisher blurb (See More), "Also grab the Audiobook with the Deepstash App" + App Store/Google Play badges, 9 hashtags.
- Stats row: "4.3/5 (0720 reviews)" [formatting bug], "Audio and text", "Download free PDF" (lead magnet).
- Then multiple competing user stashes for the same book: "Ooo Kkk" 18 ideas 223K reads (ideas: "You can change any time. You just choose not to." 3.78K likes 24.5K reads; "You fabricate emotions." etc.), "Nicholas York" 10 ideas 126K reads (idea titles: "The $12 Coffee Spill", "The Safety of Misery", "The Empty World Cure" — polished, metaphor-led titles, consistent 2-paragraph structure, reads like the newer AI-assisted/edited house style vs Dhaval Soni's rougher 2019-era notes).
- Each stash shows first 3 ideas then "Read Ideas" CTA.

## Notifications (/notifications)
- All 7 are system re-engagement pushes, none social: "Ready for your first stash of ideas? Get started" (x3 over 3 days), "🌟 Embrace new beginnings! The 5 AM Club", "🚀 Boost your potential! Do It Today", "📚 Dive back into reading!", "🏞️ Adventure awaits! Failing Forward". Cadence ≈ 2 per day. Emoji + motivational hook + a book title. Lifetime promo block repeated at bottom.

## Studio (/studio) — creator tool
- "Add New Post — Start with a blank post or pick a source": New Post / Book / Link / Video (YouTube icon) / Podcast (Spotify icon). Copy: "If you want to add ideas from a particular source, such as a book or link, add it first." Separate chrome ("Exit Studio", "Send feedback") — a distinct product surface.

## Account menu (avatar)
- View Profile / Switch to Dark Theme / Settings / Help Center (Notion site: deepstash.notion.site/Help-Center) / Sign Out / Mobile App.

## Explore (/explore)
- Chips: Highlights, Daily Picks (/explore/daily-picks), Collections (/all-collections), Books/Articles/Videos/Podcasts (/explore/sources/page/<type>), then 22 topic hashtags (/topic/<slug>).
- /explore/sources/page/videos: "Videos — 11.9K ideas". Sections READ NEXT (3 Anup Chouhan stashes from Ali Abdaal / Liam Ottley YouTube videos, 10 ideas each, 3.5–6K reads), TOP CURATORS (Anup Chouhan — bio links to a Gumroad store; Self-improvement Nerd; Peak Zenit), POPULAR (163K/153K/131K reads: Improvement Pill, Charisma on Command, Tim Ferriss), NEWEST (10/4/8 reads — new UGC gets almost no distribution).
- /explore/daily-picks: BOOK OF THE DAY (same Courage to Be Disliked stash), PREVIOUS BOOKS OF THE DAY, QUOTE OF THE DAY (Leil Lowndes), PREVIOUS QUOTES, TODAY'S PICK (How to Talk to Anyone, 6 ideas, 51K reads), PREVIOUS PICKS. Editorial daily cadence = the habit hook.
- /topic/productivity: title tag "100+ Productivity Tips & Unique Daily Facts to Win at Work" (SEO), "40877 ideas on this topic". ALL IDEAS list dominated by one curator (Anup Chouhan 4 of 5), TOP STASHERS, RELATED TOPICS. Heavy reliance on a handful of power curators.

## Collections (/all-collections)
- "START READING" grid: Productivity Hacks 109, Beat Procrastination 99, Making Remote Work Work 32, Think Outside The Box 52, Boost Your EQ 40, Ask for a Raise 64, Managing Time Like a Pro 88, Lifelong Learners 74. Only 8 visible (vs "100+ journeys" marketing). Blurred gradient covers + emoji.

## Monetisation surfaces seen (web, logged in, free account)
- Persistent "51% OFF Special Sale Lifetime Plan — Only ~~$407.15~~ $199.99. Pay once." block on home, notifications, 404 page, collections, explore pages. Benefits listed: save unlimited ideas, listen to ideas, start unlimited journeys.
- Stories (/stories): blog ("Stories Library") with tabs Collections/Articles/Stories; 7–10 min SEO listicles dated 2024 ("8 Best Spaced Repetition Apps... 2024", "Best Gamified Learning Platforms 2024") — stale content marketing.
- Dead links: /pricing 404, /businesses 404 (footer links present but broken), /collections 404.

## Settings (/settings)
- Cards: Personal Information, Preferences, Emails, Export Library ("Export your saved ideas via email"). Top-right "Manage subscription" (lock icon, external link — Stripe customer portal style). No in-app plan/price/renewal info visible on the settings page itself.
- /settings/preferences: APPEARANCE Light/Dark; "Daily reading goal banner — show a banner when you have reached your daily goal" toggle (on); the "custom reading goal" the card promises is not present on web. "Topics you Follow": 55 hashtag chips (communication…writing). /settings/emails → 404 (card exists, route missing).

## Profile (/u/stasher14332154)
- Auto-generated username "stasher14332154" (numeric suffix = user id ~14.3M, suggesting ≥14M registered accounts as of Oct 2026 — consistent with "11M+ people" claim). Tabs: Published / Saved Ideas / Stashes. Empty state "Nothing published yet… CREATE YOUR FIRST IDEA". "1/3 COMPLETE PROFILE" checklist: name & username, bio. 0 followers / 0 following.
- Saved Ideas tab lists the saved stash with every idea expanded inline (no grouping/folders unless "Stashes" used).

## Search (/search?q=stoicism&tab=ideas&owner=all)
- No autocomplete/suggestions while typing; Enter → results page with tabs Ideas / People / Topics / Sources and All / Own filter. Results = individual ideas titled "Stoicism" from 5 different user stashes (lifehack.org, britannica.com, medium.com, inc.com, thoughtco.com) — heavy duplication of the same concept across UGC; no "best answer"/canonical card. Curator handles include "Sanyasi @@".

## Collection detail (/collection/4/productivity-hacks)
- Hero illustration, emoji, title, one-line description, 4 hashtags, "Start reading" CTA. Breadcrumb says Home / Books / Productivity Hacks (wrong parent). Then a list of 10 stashes (mix of web articles from rescuetime/qz/zapier/99u/medium and user book stashes: Deep Work, Atomic Habits 2.49M reads/14.4K likes, GTD, Smarter Faster Better), each with a "0" progress marker. "Trending Collections" rail with 10 more (How to Properly Read a Book 59, Managing Perfectionism 46, Inside the Mind of Elon Musk 75…). Collection idea counts differ between surfaces (Managing Time Like a Pro: 88 on /all-collections vs 53 here; Think Outside the Box 52 vs 31).
- "Start reading" simply opens the first stash with ?collection=4 appended. On the stash page there is NO collection progress bar, no "next in collection" control, no completion state — the journey mechanic (progress, quizzes) is mobile-only.

## Stash page actions (web)
- Listen (headphones) → modal "Get the app to listen to 200,000+ ideas" with App Store / Google Play badges, "3M+ Installs" and "4.7 App Rating" — audio is not available on web at all; the modal uses the stale 3M figure.
- Save (bookmark) → saves instantly to the account (no limit messaging on first save); icon turns into "MOVE" (move to a stash). [Note: one stash — The Courage to Be Disliked — was saved to the user's account during this teardown.]
- "..." menu → Report / Visit Original only (no share-to-X/LinkedIn, no copy-link beyond the share icon).
- Idea cards: like count + bookmark, "N reads", kebab; no comments per idea (comments are per stash, count 2–122).

## Video-sourced stash (/article/326244/how-to-make-your-first-1000-online)
- "Curated from: Ali Abdaal" with a youtu.be link; 10 numbered ideas each exactly 3 one-sentence lines (formulaic template), 5.37K reads, declining 1.2K→282 reads by idea 9 (≈77% drop). Idea 5 contains a sales pitch: "grab 500+ Side Hustle Ideas—it filters opportunities by time, budget and skill level" → the curator (Anup Chouhan, 5,158 ideas, 740K saves, 6,598 followers, Gumroad store in bio) is inserting affiliate/product promotion inside idea text. Newest UGC in the Videos feed has 4–10 reads.
- Direct URL navigation to /article/* pages hits a Cloudflare bot challenge; in-app navigation does not.

## Studio editor (/studio/draft/326648)
- Creating a "New Post" immediately creates a draft ("Untitled (1)"). Left: cover "Add Image"; DRAFT STATUS checklist: Title added ✔ / Image missing / Context missing / No ideas added / No empty ideas; PUBLISH button (disabled until checklist passes). Body: "Add a context for this post…" quote field, then block types Text / Quote / Image. No AI assist, no source-import for a blank post, no scheduling. [A blank draft now exists in the user's Studio.]

## Lifetime offer → checkout
- "Claim Lifetime Offer" → buy.stripe.com Payment Link (Brainstash, Inc.), client_reference_id = user id, email locked/prefilled, Card + Cash App Pay + Link, "Payment mode: Live mode". Purchase happens on Stripe, outside Apple/Google — matches reviews saying web purchases don't appear in Apple subscriptions. No plan summary/terms text in the main pane captured; price shown in-app: $199.99 one-time (anchored at $407.15, "51% OFF").

## Cross-cutting observations
- Inconsistent install/user numbers in one session: About page 3M+, listen modal 3M+, homepage 10M+, App Store copy 11M+, user ids ~14.3M.
- Three distinct product eras coexist: 2019-era UGC notes (Dhaval Soni), 2020–22 web-article clippings (rescuetime/qz), and 2025–26 templated 10-idea book/video stashes by power curators (Anup Chouhan) — the feed mixes them without signalling quality tier.
- Web app is a read + save + publish surface; audio, offline, journeys/progress, quizzes, streaks, reading goal and the Pro paywall proper are mobile-only, with web deflecting to store badges.
- Broken/dead routes from the global footer: /pricing, /businesses, /collections, /settings/emails.
