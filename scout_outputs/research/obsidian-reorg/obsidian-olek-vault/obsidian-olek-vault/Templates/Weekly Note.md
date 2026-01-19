---
created: <% tp.date.now("YYYY-MM-DD") %>
type: weekly
week: <% tp.date.now("YYYY-[W]ww") %>
tags: [weekly]
---

# Week <% tp.date.now("ww") %> - <% tp.date.now("MMMM YYYY") %>

<< [[<% tp.date.now("YYYY-[W]ww", -7) %>]] | [[<% tp.date.now("YYYY-[W]ww", 7) %>]] >>

---

## 📅 This Week's Daily Notes
```dataview
LIST
FROM "01-Daily"
WHERE file.cday >= date("<% tp.date.now("YYYY-MM-DD", "P-6D") %>") AND file.cday <= date("<% tp.date.now("YYYY-MM-DD") %>")
SORT file.name ASC
```

## 🎯 Weekly Goals
- [ ] 
- [ ] 
- [ ] 

## 📊 All Tasks This Week
```dataview
TASK
FROM "01-Daily"
WHERE file.cday >= date("<% tp.date.now("YYYY-MM-DD", "P-6D") %>") AND file.cday <= date("<% tp.date.now("YYYY-MM-DD") %>")
WHERE !completed
GROUP BY file.link
```

## ✅ Completed This Week
```dataview
TASK
FROM "01-Daily"
WHERE file.cday >= date("<% tp.date.now("YYYY-MM-DD", "P-6D") %>") AND file.cday <= date("<% tp.date.now("YYYY-MM-DD") %>")
WHERE completed
GROUP BY file.link
```

## 🔗 Notes Modified This Week
```dataview
TABLE file.mtime as "Last Modified"
WHERE file.mday >= date("<% tp.date.now("YYYY-MM-DD", "P-6D") %>")
WHERE !contains(file.path, "Templates")
SORT file.mtime DESC
LIMIT 25
```

## 💭 Week Reflection
**What went well:**


**What could improve:**


**Next week focus:**

