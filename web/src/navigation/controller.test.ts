import assert from "node:assert/strict";
import { test } from "node:test";
import { createNavigation } from "./controller.ts";

const origin = "https://rustsurvey.example/";

function setup(route = "#/responses/synthetic-response/synthetic-secret") {
  let current = `${origin}${route}`;
  const replacements: string[] = [];
  const navigation = createNavigation({
    readUrl: () => current,
    replaceUrl: (url) => {
      current = url;
      replacements.push(url);
    },
  });
  function visit(route: string): string | null {
    current = `${origin}${route}`;
    return navigation.change(current);
  }
  return {
    navigation,
    visit,
    replacements,
    url: () => current,
    move: (route: string) => {
      current = `${origin}${route}`;
    },
  };
}

test("test_navigation_should_keep_the_form_and_restore_its_private_url_when_leaving_is_cancelled", () => {
  const { navigation, visit, url } = setup();
  const before = url();
  navigation.protect(() => false);
  assert.equal(visit("#/admin"), null);
  assert.equal(url(), before);
  assert.equal(navigation.route(), "#/responses/synthetic-response/synthetic-secret");
});

test("test_navigation_should_follow_links_or_history_when_leaving_is_allowed", () => {
  const { navigation, visit, url } = setup();
  let allowed = false;
  navigation.protect(() => allowed);
  assert.equal(visit("#/"), null);
  allowed = true;
  assert.equal(visit("#/"), "#/");
  assert.equal(url(), `${origin}#/`);
  assert.equal(
    visit("#/responses/synthetic-response/synthetic-secret"),
    "#/responses/synthetic-response/synthetic-secret",
  );
});

test("test_navigation_should_release_a_guard_when_its_form_is_unmounted", () => {
  const { navigation, visit } = setup();
  const release = navigation.protect(() => false);
  release();
  assert.equal(visit("#/import"), "#/import");
});

test("test_navigation_should_restore_the_canonical_resume_url_after_a_cancelled_back_action", () => {
  const { navigation, visit, url } = setup("#/participate/study/synthetic-secret");
  assert.equal(
    navigation.replaceHash(
      "#/participate/study/synthetic-secret",
      "#/responses/response/synthetic-secret",
    ),
    true,
  );
  navigation.protect(() => false);
  assert.equal(visit("#/studies/study"), null);
  assert.equal(url(), `${origin}#/responses/response/synthetic-secret`);
  assert.equal(navigation.change(url()), null, "canonicalization must not remount the form");
});

test("test_navigation_should_ignore_stale_history_events_and_restore_the_last_rendered_route", () => {
  const { navigation, move, url } = setup();
  const before = url();
  let prompts = 0;
  navigation.protect(() => {
    prompts += 1;
    return false;
  });
  move("#/admin");
  move("#/import");
  assert.equal(navigation.change(`${origin}#/admin`), null);
  assert.equal(prompts, 0);
  assert.equal(navigation.change(`${origin}#/import`), null);
  assert.equal(prompts, 1);
  assert.equal(url(), before);
});

test("test_navigation_should_not_rewrite_a_new_page_when_a_previous_start_request_finishes_late", () => {
  const { navigation, visit, url, move } = setup("#/participate/study/synthetic-secret");
  visit("#/admin");
  assert.equal(
    navigation.replaceHash(
      "#/participate/study/synthetic-secret",
      "#/responses/response/synthetic-secret",
    ),
    false,
  );
  assert.equal(url(), `${origin}#/admin`);
  move("#/import");
  assert.equal(
    navigation.replaceHash("#/admin", "#/responses/response/synthetic-secret"),
    false,
    "a hash change awaiting dispatch also owns its URL",
  );
});
