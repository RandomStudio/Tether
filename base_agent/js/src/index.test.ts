import { topicMatchesPlug } from ".";

describe("matching topics to plugs", () => {
  test("if Plug specified full topic, i.e. no wildcards, then only exact matches", () => {
    const plugDefinedTopic = "someType/someGroup/somePlugName";

    expect(
      topicMatchesPlug(plugDefinedTopic, "someType/someGroup/somePlugName")
    ).toBeTruthy();

    expect(
      topicMatchesPlug(plugDefinedTopic, "other/otherGroup/somePlugName")
    ).toBeFalsy();
  });

  test("if ONLY Plug Name specified, match any with same PlugName", () => {
    const plugDefinedTopic = "+/+/somePlugName";
    expect(
      topicMatchesPlug(plugDefinedTopic, "something/something/somePlugName")
    ).toBeTruthy();
  });

  test("if ONLY Plug Name specified, match any with same PlugName", () => {
    const plugDefinedTopic = "+/+/somePlugName";
    expect(
      topicMatchesPlug(plugDefinedTopic, "something/something/somePlugName")
    ).toBeTruthy();
  });

  test("if AgentType and PlugName specified, but not GroupOrId, then match when these match", () => {
    const plugDefinedTopic = "specificAgent/+/plugName";

    expect(
      topicMatchesPlug(plugDefinedTopic, "specificAgent/anything/plugName")
    ).toBeTruthy();
    expect(
      topicMatchesPlug(plugDefinedTopic, "specificAgent/somethingElse/plugName")
    ).toBeTruthy();
    expect(
      topicMatchesPlug(plugDefinedTopic, "differentAgent/anything/plugName")
    ).toBeFalsy();
  });

  test("if GroupOrId and PlugName specified, but not AgentType, then match when these match", () => {
    const plugDefinedTopic = "+/specificGroupOrId/plugName";

    expect(
      topicMatchesPlug(plugDefinedTopic, "someAgent/specificGroupOrId/plugName")
    ).toBeTruthy();
    expect(
      topicMatchesPlug(
        plugDefinedTopic,
        "anotherAgent/specificGroupOrId/plugName"
      )
    ).toBeTruthy();
    expect(
      topicMatchesPlug(plugDefinedTopic, "someAgent/wrongGroup/plugName")
    ).toBeFalsy();
  });

  test("if Plug Name was never specified, throw an Error", () => {
    const plugDefinedTopic = "something/something/+";
    try {
      expect(
        topicMatchesPlug(plugDefinedTopic, "anything/anything/anything")
      ).toThrow();
    } catch (e) {
      //
    }
  });
});
