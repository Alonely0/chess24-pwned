import fs from "fs";
import path from "path";
import { Readable } from "stream";
import { finished } from "stream/promises";
import { handle, req } from "./index.js";

const videoSelector = "video.fp-engine[src]";

export const scrapeVideo = async (page, url, dir) => {
  let res = [];
  await intercept(page, res);

  await req(page, "https://chess24.com/es");
  // await page.setViewport({ width: 1920, height: 1080 });
  await page.waitForNetworkIdle(1000, 2000);
  await req(page, url);
  await clickPlay(page);
  await page.waitForSelector(videoSelector);

  const videoUrl = await getVideoUrl(page);
  fs.writeFile(
    path.resolve(dir, "res.json"),
    JSON.stringify([...(await videoUrl), ...res]),
    handle
  );
  await Promise.all([
    download(videoUrl, path.resolve(dir, "video.webm")),
    writeChessBoardFiles(page, res, dir),
  ]);
  await page.removeAllListeners("request");
  console.log(`[${process.argv[3]}]: done ${url}`);
  // await new Promise(x => setTimeout(x, 2000));
};

const intercept = async (page, res) => {
  await page.setRequestInterception(true);
  page.on("request", (request) => {
    request.continue();
    const url = request.url();
    if (
      url.startsWith(
        "https://chess24.com/api/web/videoSeriesAPI/videoDescription"
      )
    ) {
      if (res.indexOf(url) === -1) res.push(url);
    }
  });
};

const clickPlay = async (page) => {
  const playBtn = ".fp-play.gameVideoControlIcon.iconPlay";
  await page.waitForSelector(playBtn);
  await page.click(playBtn);
};

const writeChessBoardFiles = async (page, res, dir) => {
  fs.writeFile(path.resolve(dir, "res.json"), JSON.stringify(res), (x) => {});
  for (var i = 0; i < res.length; i++) {
    await page.goto(res[i]);
    fs.writeFile(
      path.resolve(dir, `${i}.json`),
      await page.$eval("*", (x) => x.innerText),
      (x) => {}
    );
  }
};

async function arq(prom) {
  try {
    prom();
  } catch {
    await new Promise((x) => setTimeout(x, 2000 + Math.random() * 500));
    arq(prom);
  }
}

const getVideoUrl = async (page) => {
  return await page.$$eval(videoSelector, (x) =>
    x.map((y) => y.getAttribute("src"))
  );
};

const download = async (url, dest) => {
  try {
    const res = await fetch(url);
    const fileStream = fs.createWriteStream(dest, {
      flags: "wx",
      flush: true,
    });
    await finished(Readable.fromWeb(res.body).pipe(fileStream));
  } catch {
    fs.unlink(dest, async (_err) => await download(url, dest));
  }
};
