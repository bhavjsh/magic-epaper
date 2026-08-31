import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:magicepaperapp/constants/dimens.dart';
import 'package:magicepaperapp/theme/colors.dart';
import 'package:magicepaperapp/view/widget/common_scaffold_widget.dart';

class _FaqItem {
  final String question;
  final String answer;
  const _FaqItem(this.question, this.answer);
}

const _faqs = [
  _FaqItem(
    'What is an ePaper badge and how does this app work with it?',
    'An ePaper badge is a small wireless display that uses electronic ink — the same technology as e-readers. It shows crisp images without needing constant power. Magic ePaper lets you design content on your phone and transfer it to the badge wirelessly using NFC — just tap your phone against the badge and the image is written instantly.',
  ),
  _FaqItem(
    'Why do I need to select a display type before I start designing?',
    'Every ePaper badge has a fixed screen size and a specific set of supported colors. Selecting your display ensures your design is sized correctly and uses only the colors your badge can actually show. Choosing the wrong display will result in a stretched, cropped, or incorrectly colored image on the badge.',
  ),
  _FaqItem(
    'What can I create with this app?',
    'You can create almost anything that fits on your badge — import and dither photos, draw freehand and add text or shapes on the canvas, generate QR codes and barcodes, or use ready-made card templates like Employee ID, Price Tag, Event Badge, Calendar, and more. You can also generate many cards at once from a CSV file.',
  ),
  _FaqItem(
    'Do I need cables or internet to transfer a design to my badge?',
    'No cables needed — the transfer happens over NFC, so you just hold the back of your phone against the badge. Internet is not required for most features. The only exception is the Weather Snapshot template, which fetches live weather data.',
  ),
  _FaqItem(
    'Why does my photo look different on the badge than on my phone?',
    'ePaper displays can only show a limited set of colors — usually black and white, or black, white, red, and yellow. A regular photo has millions of colors, so the app uses a process called dithering to convert it into the colors your badge supports. Try different dithering algorithms (Floyd–Steinberg, Atkinson, etc.) in the image editor to get the best result for your photo.',
  ),
  _FaqItem(
    'How do I know if the transfer was successful?',
    'The app shows a success message once the transfer is complete, and your badge will begin refreshing — you\'ll see it flash briefly before the new image appears. If the transfer fails, the app will notify you and your badge will continue showing its previous image unchanged.',
  ),
  _FaqItem(
    'Does the badge need to be charged or plugged in?',
    'No. ePaper badges are completely passive — they draw the power they need directly from your phone\'s NFC field during the transfer. Once the image is written, it stays on the display indefinitely with zero power consumption, even if the badge is disconnected.',
  ),
  _FaqItem(
    'Can I update or change the design on my badge later?',
    'Yes, as many times as you want. Simply create a new design in the app and transfer it to the badge again. You can also save designs to the Image Library for quick re-transfer later. There is no limit to how many times you can update a badge.',
  ),
  _FaqItem(
    'Does the app work on iPhone?',
    'Not yet. iOS restricts the kind of NFC communication needed to write images to ePaper badges, so the transfer feature is Android-only for now. The design tools — canvas, templates, and image editor — are available on desktop builds which are currently in development.',
  ),
  _FaqItem(
    'My badge isn\'t responding when I tap my phone — what should I try?',
    'First, make sure NFC is enabled on your phone under Settings → Connected devices → NFC. Hold the back of your phone flat and still against the badge — the NFC antenna is usually near the top of the phone. Avoid phone cases with metal plates as they can block NFC. If transfers keep failing, try disabling battery-saving mode, as it can interrupt NFC.',
  ),
];

class FaqScreen extends StatelessWidget {
  const FaqScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return CommonScaffold(
      index: 9,
      title: 'FAQs',
      body: SafeArea(
        top: false,
        bottom: true,
        child: ListView.separated(
          padding: const EdgeInsets.all(Dimens.spacingS),
          itemCount: _faqs.length,
          separatorBuilder: (_, __) =>
              const SizedBox(height: Dimens.spacingMd),
          itemBuilder: (context, i) => _FaqTile(item: _faqs[i]),
        ),
      ),
    );
  }
}

class _FaqTile extends StatelessWidget {
  final _FaqItem item;
  const _FaqTile({required this.item});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: colorWhite,
        borderRadius: BorderRadius.circular(Dimens.radiusS),
        boxShadow: const [
          BoxShadow(
            color: grey500,
            offset: Offset(0, 1),
            blurRadius: 2.0,
          ),
        ],
      ),
      child: ExpansionTile(
        tilePadding: const EdgeInsets.symmetric(
          horizontal: Dimens.spacingM,
          vertical: Dimens.spacingS,
        ),
        childrenPadding: const EdgeInsets.fromLTRB(
          Dimens.spacingM,
          0,
          Dimens.spacingM,
          Dimens.spacingM,
        ),
        title: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Q: ',
              style: GoogleFonts.sora(
                color: colorAccent,
                fontWeight: FontWeight.w600,
                fontSize: Dimens.fontSizeM,
              ),
            ),
            Expanded(
              child: Text(
                item.question,
                style: GoogleFonts.sora(
                  color: colorAccent,
                  fontWeight: FontWeight.w600,
                  fontSize: Dimens.fontSizeM,
                ),
              ),
            ),
          ],
        ),
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'A: ',
                style: GoogleFonts.sora(
                  color: colorBlack,
                  fontWeight: FontWeight.w500,
                  fontSize: Dimens.fontSizeS,
                ),
              ),
              Expanded(
                child: Text(
                  item.answer,
                  style: GoogleFonts.sora(
                    color: colorBlack,
                    fontWeight: FontWeight.w400,
                    fontSize: Dimens.fontSizeS,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
