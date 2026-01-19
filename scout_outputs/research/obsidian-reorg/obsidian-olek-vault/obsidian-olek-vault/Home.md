---
created: 2025-01-18
type: dashboard
tags: [dashboard, home]
---

# 🏠 Home

> Press `Cmd+D` to open today's daily note

---

## ⚡ Quick Actions
- [[01-Daily|Daily Notes →]]
- [[02-Work/Catsy/Dashboard|Catsy Dashboard →]]
- [[02-Work/Arkai/Dashboard|Arkai Dashboard →]]

---

## 🔥 All Urgent Tasks
```dataview
TASK
WHERE contains(text, "#urgent") AND !completed
GROUP BY file.link
LIMIT 10
```

## 📋 Today's Tasks
```dataview
TASK
FROM "01-Daily"
WHERE file.day = date(today) AND !completed
```

## 📝 Modified Today
```dataview
LIST
WHERE file.mday = date(today)
WHERE !contains(file.path, "Templates")
SORT file.mtime DESC
LIMIT 10
```

## 📅 Recent Daily Notes
```dataview
LIST
FROM "01-Daily"
SORT file.name DESC
LIMIT 7
```

---

## 🔗 Graph Visualization
Open the Graph View (`Cmd/Ctrl + G`) to see connections between notes.

## 📊 Vault Stats
- Total notes: `$= dv.pages().length`
- Notes this week: `$= dv.pages().where(p => p.file.mday >= dv.date('today') - dv.duration('7 days')).length`
