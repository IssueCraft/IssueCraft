Feature: Query feature

  Background:
    Given a fresh database
    And a single user authorization provider

  Scenario: A fresh database always has a default user
    Then a user "default" exists with the name "Default User"

  Scenario: A project exists after creation
    When I create a project "test" with the display name "Test Project"
    Then a project "test" exists with the name "Test Project"

  Scenario: A project can be renamed
    When I create a project "test" with the display name "Test Project"
    And I update the display name of the project "test" to "Renamed Project"
    Then a project "test" exists with the name "Renamed Project"

  Rule: An issue can only be created for a project

    Background:
        When I create a project "test" with the display name "Test Project"

    Scenario Outline: An issue has a sequential id, prefixed by the project id, followed by a #
        When I create an issue of kind "<kind>" with the title "<title>" in project "test"
        And I create an issue of kind "<kind2>" with the title "<title2>" in project "test"
        Then an issue "test#1" exists with the kind "<kind>" and title "<title>"
        Then an issue "test#2" exists with the kind "<kind2>" and title "<title2>"

    Examples:
        | kind        | title            | kind2       | title2             |
        | bug         | Test Bug         | bug         | Test Bug 2         |
        | improvement | Test Improvement | improvement | Test Improvement 2 |
        | task        | Test Task        | bug         | Test Bug 4         |

  Rule: A comment can only be created for an issue

    Background:
        When I create a project "test" with the display name "Test Project"
        And I create an issue of kind "bug" with the title "Test Bug" in project "test"

    Scenario: A comment is created with an id starting with 'C'
        When I comment "Test Comment" on issue "test#1"
        Then a comment exists with author "default", issue id "test#1" and content "Test Comment"

