import puppeteer from "puppeteer-extra";
import stealth from "puppeteer-extra-plugin-stealth";
import fs from "fs";
import path from "path";
import { scrapeVideo } from "./videoScraper.js";
import { scrapeCourses } from "./courseScraper.js";
import { scrapeChapters } from "./chapterScraper.js";

export const handle = (err) => {
  if (err != null) console.log(err);
};

const loadCookie = async (page) => {
  fs.readFile("./cookies.json", async (err, cookieJson) => {
    if (err != null) {
      console.log(err);
    } else {
      const cookies = JSON.parse(cookieJson.toString());
      await page.setCookie(...cookies);
    }
  });
};

const saveCookie = async (page) => {
  const cookies = await page.cookies();
  const cookieJson = JSON.stringify(cookies);
  fs.writeFileSync("cookies.json", cookieJson);
};

const main = async () => {
  puppeteer.use(stealth); // evade headless chromium detection
  const browser = await puppeteer.launch({ headless: "new" });
  const page = await browser.newPage();
  await loadCookie(page);
  await req(page, "https://chess24.com/es");
  await page.waitForNetworkIdle(1000, 15000);
  // await page.setViewport({width: 1920, height: 1080});
  // await new Promise((x) => setTimeout(x, 60000));
  // await saveCookie(page);

  await doTheJob(browser, page);
  await browser.close();
};

const doTheJob = async (browser, page) => {
  page.setDefaultNavigationTimeout(0);
  let courses;
  const coursesPath = path.resolve(process.argv[4], "courses.json");
  try {
    courses = JSON.parse(fs.readFileSync(coursesPath).toString());
  } catch {
    courses = await scrapeCourses(page);
    fs.writeFileSync(coursesPath, JSON.stringify(courses));
  }

  // For use with GNU parallel. Elsewhere, provide `1 0 <path>` as args
  courses = chunks(courses, Math.ceil(courses.length / process.argv[2]))[
    process.argv[3]
  ];
  for (const c of courses) {
    const videos = await scrapeChapters(page, c, process.argv[4]);
    for (const [url, dir] of videos) {
      const tab = await browser.newPage();
      await loadCookie(tab);
      fs.mkdir(dir, { recursive: true }, handle);
      await sv(tab, url, dir);
      await tab.close();
    }
  }
};

async function sv(a, b, c) {
  try {
    await scrapeVideo(a, b, c);
  } catch {
    await a.removeAllListeners("request");
    await sv(a, b, c);
  }
}

const chunks = (arr, chunkSize) => {
  const res = [];
  for (let i = 0; i < arr.length; i += chunkSize) {
    const chunk = arr.slice(i, i + chunkSize);
    res.push(chunk);
  }
  return res;
};

// For addling dynamic throttling if needed
export const req = async (page, url) => {
  try {
    await page.goto(url, { waitUntil: "load" });
  } catch {
    await req(page, url);
  }
};

(async () => await main())();
