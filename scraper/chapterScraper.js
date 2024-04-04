import path from "path";
import { req } from "./index.js";

const chapterSelector = ".videosList > ul > li > h3 > a[href]";
const lastPartRegex = /[^/]+(?=\/$|$)/;

export const scrapeChapters = async (page, url, dir) => {
  await req(page, url);
  const chName = path.resolve(dir, url.match(lastPartRegex)[0]);
  await new Promise((x) => setTimeout(x, 1000));
  let i = 0;
  return [
    ...(await page.$$eval(chapterSelector, (x) =>
      x.map((y) => {
        const url = "https://chess24.com" + y.getAttribute("href");
        return url;
      })
    )),
  ].map((url) => {
    i++;
    return [url, path.resolve(chName, `${i}. ${url.match(lastPartRegex)[0]}`)];
  });
};
