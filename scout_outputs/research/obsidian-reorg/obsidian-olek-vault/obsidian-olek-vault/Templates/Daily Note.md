---
created: <% tp.date.now("YYYY-MM-DD") %>
type: daily
tags: [daily]
---

# <% tp.date.now("dddd, MMMM Do YYYY") %>

<< [[<% tp.date.now("YYYY-MM-DD", -1) %>]] | [[<% tp.date.now("YYYY-MM-DD", 1) %>]] >>

---

## ☀️ Top 3 Today
- [ ] 
- [ ] 
- [ ] 

## 📋 Tasks
- [ ] 

## 📝 Notes


## 💡 Ideas


---

## 🔗 Modified Today
```dataview
LIST
WHERE file.mday = this.file.day
WHERE file.name != this.file.name
SORT file.mtime DESC
LIMIT 10
```

## ✅ Completed Today
```dataview
TASK
WHERE completed
WHERE completion = this.file.day
LIMIT 20
```
