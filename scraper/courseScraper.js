import assert from "assert";
import { req } from "./index.js";

export const scrapeCourses = async (page) => {
  await req(page, "https://chess24.com/es/aprende/videos?lang=es");
  await page.waitForNetworkIdle(1000, 15000);
  await page.click("div.selectedFilters > a.cBtn");
  await page.waitForNetworkIdle(1000, 15000);
  const pages = await getPages(page);
  const courses = [];
  for (var i = 2; i < Infinity; i++) {
    courses.push(...(await getCourses(page)));
    try {
      await page.click(`li.page>a[href$="=${i}"]`);
    } catch {
      break;
    }
    await page.waitForNetworkIdle(1000, 15000);
  }
  assert(courses.length > (pages - 1) * 20);
  return courses;
};

const getPages = async (page) => {
  return (
    await page.$eval("li.goLast>a[href]", (btn) => btn.getAttribute("href"))
  ).match(/\d+$/)[0];
};

const getCourses = async (page) => {
  return await page.$$eval("a.learnItemBoxLink[href]", (x) =>
    x.map((y) => "https://chess24.com" + y.getAttribute("href"))
  );
};
