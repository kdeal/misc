import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';

type NewPicture = {
  imageSrc: string;
  location: string;
  timeTaken: string;
};

let mainImageEl: HTMLImgElement | null;
let currentImage: NewPicture | null;

listen<NewPicture>('new-picture', (event) => {
  currentImage = event.payload;
  mainImageEl.src = currentImage.imageSrc;
});

window.addEventListener("DOMContentLoaded", () => {
  mainImageEl = document.querySelector("#main-image");
});
