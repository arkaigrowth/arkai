---
created: 2025-01-18
type: dashboard
tags: [dashboard, catsy]
---

# 🏢 Catsy Dashboard

## 🔥 Urgent Tasks
```dataview
TASK
FROM "01-Daily" OR "02-Work/Catsy"
WHERE contains(text, "#catsy/urgent") AND !completed
SORT file.mtime DESC
```

## 📋 All Open Catsy Tasks
```dataview
TASK
FROM "01-Daily" OR "02-Work/Catsy"
WHERE contains(text, "#catsy") AND !completed
GROUP BY file.link
```

## 📅 Recent Activity
```dataview
TABLE file.mtime as "Modified"
FROM "02-Work/Catsy" OR (#catsy AND "01-Daily")
WHERE file.mday >= date(today) - dur(7 days)
SORT file.mtime DESC
LIMIT 15
```

## 🔗 Quick Links
- [[Catsy Clients]]
- [[Catsy API Notes]]
- [[Support Playbook]]

## 👥 Key People
- [[CJ]]
- [[Jamie]]
- [[Jarod]]
