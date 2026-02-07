Feature: Query feature

  Background:
    Given a fresh database
    And a single user provider
    And a single user authorization provider

  Scenario: A fresh database always has a default user
    Then a user 'default' exists with the name 'Default User' and the email 'default@example.com'

  Scenario: A project exists after creation
    When I create a project "test" with the display name "Test Project"
    Then a project "test" exists with the name "Test Project"

  Scenario: A project can be renamed
    When I create a project "test" with the display name "Test Project"
    And I update the display name of the project "test" to "Renamed Project"
    Then a project 'test' exists with the name 'Renamed Project'
