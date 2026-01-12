Require echo

Set Shell "bash"
Set FontSize 22
Set Width 1200
Set Height 600

Type `issuecraft "CREATE PROJECT test WITH NAME 'Test Project'"`
Sleep 500ms
Enter

Type `issuecraft "SELECT * FROM projects"`
Sleep 500ms
Enter
Sleep 1500ms

Type `issuecraft "CREATE ISSUE OF KIND bug IN test WITH TITLE 'Something is wrong'"`
Sleep 500ms
Enter

Type `issuecraft "SELECT * FROM issues"`
Sleep 500ms
Enter
Sleep 1500ms

Type `issuecraft "UPDATE ISSUE test#1 SET description = 'This is a test issue'"`
Sleep 500ms
Enter

Type `issuecraft "SELECT * FROM issues"`
Sleep 500ms
Enter
Sleep 1500ms

Type `issuecraft "COMMENT ON ISSUE test#1 WITH 'Some Comment'"`
Sleep 500ms
Enter

Type `issuecraft "SELECT * FROM comments WHERE issue = 'test#1'"`
Sleep 500ms
Enter

Sleep 5s
