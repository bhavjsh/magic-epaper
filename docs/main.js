const lightbox = document.getElementById('lightbox');
const lbImg = document.getElementById('lb-img');

document.querySelectorAll('.gallery figure').forEach(fig => {
  fig.addEventListener('click', () => {
    const img = fig.querySelector('img');
    const cap = fig.querySelector('figcaption');
    lbImg.src = img.src;
    lbImg.alt = cap ? cap.textContent : '';
    lightbox.classList.add('open');
  });
});

lightbox.querySelector('.lb-close').addEventListener('click', () => lightbox.classList.remove('open'));
lightbox.addEventListener('click', e => { if (e.target === lightbox) lightbox.classList.remove('open'); });
document.addEventListener('keydown', e => { if (e.key === 'Escape') lightbox.classList.remove('open'); });
