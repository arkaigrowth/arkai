---
created: 2025-01-18
type: dashboard
tags: [dashboard, arkai]
---

# 🚀 Arkai Growth Dashboard

## 📋 Open Tasks
```dataview
TASK
FROM "01-Daily" OR "02-Work/Arkai"
WHERE contains(text, "#arkai") AND !completed
GROUP BY file.link
```

## 💡 Ideas & Leads
```dataview
LIST
FROM "02-Work/Arkai"
WHERE contains(tags, "idea") OR contains(tags, "lead")
SORT file.mtime DESC
LIMIT 10
```

## 📅 Recent Activity
```dataview
TABLE file.mtime as "Modified"
FROM "02-Work/Arkai" OR (#arkai AND "01-Daily")
WHERE file.mday >= date(today) - dur(14 days)
SORT file.mtime DESC
LIMIT 15
```

## 🔗 Quick Links
- [[Arkai Services]]
- [[Client Proposals]]
- [[Automation Templates]]
