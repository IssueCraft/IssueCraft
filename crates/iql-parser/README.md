# IssueCraft Query Language (IQL)

A parser for a simple, SQL-like, language to provide a nicer way to interact with the
system through the IssueCraft CLI.

## IQL Description

```sql
CREATE Statements:
  CREATE USER <username> [WITH EMAIL <email> NAME '<name>']
  CREATE PROJECT <project-id> [WITH NAME '<name>' DESCRIPTION '<desc>' OWNER <username>]
  CREATE ISSUE IN <project> WITH TITLE '<title>' [DESCRIPTION '<desc>'] [PRIORITY <level>] [ASSIGNEE <user>] [LABELS [<label>, ...]]
  CREATE COMMENT ON ISSUE <id> WITH '<content>' [AUTHOR <username>]

SELECT Statements:
  SELECT * FROM <entity>
  SELECT <col1>, <col2>, ... FROM <entity>
  SELECT ... WHERE <condition>
  SELECT ... ORDER BY <field> [ASC|DESC]
  SELECT ... LIMIT <n> [OFFSET <n>]

UPDATE Statements:
  UPDATE <entity-type> <id> SET <field> = <value>[, ...]

DELETE Statements:
  DELETE <entity-type> <id>

Other Statements:
  ASSIGN ISSUE <id> TO <username>
  CLOSE ISSUE <id> [WITH '<reason>']
  COMMENT ON ISSUE <id> WITH '<content>'

Entity Types: USER, PROJECT, ISSUE, USERS, PROJECTS, ISSUES, COMMENTS
Priority Levels: critical, high, medium, low
Issue ID format: <project#number> (e.g., 'PROJ#123')
```