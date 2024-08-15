use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedReadHalf, TcpStream},
    sync::Mutex,
};

use super::backend::{fetch_incoming_message_lenght, get_image_header, Application, ClientMessage};
pub const VOIP_PACKET_BUFFER_LENGHT_MS: usize = 35;

use dashmap::DashMap;
use image::ImageOutputFormat;
use indexmap::IndexMap;
use rodio::Sink;
use std::{
    collections::{HashMap, VecDeque},
    io::{BufReader, BufWriter, Cursor},
    sync::{mpsc, Arc},
    time::Duration,
};
use tokio::select;

use crate::app::backend::{decrypt_aes256_bytes, ImageHeader, MessageBuffer, UdpMessageType};

use crate::app::ui::client_ui::client_actions::audio_recording::{
    create_wav_file, record_audio_with_interrupt,
};

/// Sends connection request to the specified server handle, returns the server's response, this function does not create a new thread, and may block
pub async fn connect_to_server(
    mut connection: TcpStream,
    message: ClientMessage,
) -> anyhow::Result<(String, TcpStream)>
{
    let message_as_string = message.struct_into_string();

    let message_bytes = message_as_string.as_bytes();

    //Send message lenght to server
    connection
        .write_all(&(message_bytes.len() as u32).to_be_bytes())
        .await?;

    //Send message to server
    connection.write_all(message_bytes).await?;

    //Read the server reply lenght
    //blocks here for unknown reason
    let msg_len = fetch_incoming_message_lenght(&mut connection).await?;

    //Create buffer with said lenght
    let mut msg_buffer = vec![0; msg_len as usize];

    //Read the server reply
    connection.read_exact(&mut msg_buffer).await?;

    Ok((String::from_utf8(msg_buffer)?, connection))
}

pub struct ServerReply
{
    pub reader: Arc<Mutex<OwnedReadHalf>>,
}

impl ServerReply
{
    pub async fn wait_for_response(&self) -> anyhow::Result<String>
    {
        let reader = &mut *self.reader.lock().await;

        // Read the server reply lenght
        let msg_len = fetch_incoming_message_lenght(reader).await?;

        //Create buffer with said lenght
        let mut msg_buffer = vec![0; msg_len as usize];

        //Read the server reply
        reader.read_exact(&mut msg_buffer).await?;

        Ok(String::from_utf8(msg_buffer)?)
    }

    pub fn new(reader: Arc<Mutex<OwnedReadHalf>>) -> Self
    {
        Self { reader }
    }
}

impl Application
{
    ///This function is used to send voice recording in a voip connection, this function spawns a thread which record 35ms of your voice then sends it to the linked voip destination
    pub fn client_voip_thread(&mut self, ctx: &egui::Context)
    {
        if let Some(voip) = self.client_ui.voip.clone() {
            self.voip_thread.get_or_insert_with(|| {
                let uuid = self.opened_user_information.uuid.clone();
                let destination = self.client_ui.send_on_ip.clone();
                let decryption_key = self.client_connection.client_secret.clone();
                let cancel_token = self.voip_shutdown_token.clone();
                let cancel_token_child = cancel_token.child_token();

                let reciver_socket_part = voip.socket.clone();
                let microphone_precentage = self.client_ui.microphone_volume.clone();

                let (tx, rx) = mpsc::channel::<()>();

                self.record_audio_interrupter = tx;

                let uuid_clone = uuid.clone();
                let decryption_key_clone = decryption_key.clone();
                let voip_clone = voip.clone();
                let camera_handle = voip_clone.camera_handle.clone();
                let cancel_token_clone = self.webcam_recording_shutdown.clone();
                //Create image sender thread
                tokio::spawn(async move {
                    loop {
                        select! {
                            //Lock camera handle
                            mut camera_handle = camera_handle.lock() => {
                                //Get image bytes from the cameras
                                match camera_handle.as_mut() {
                                    Some(handle) => {
                                        //Create buffer for image
                                        let mut buffer = BufWriter::new(Cursor::new(Vec::new()));
                                        //Get camera frame
                                        let (camera_bytes, size) = handle.get_frame().unwrap_or_default();

                                        //Convert raw image bytes to jpeg
                                        image::write_buffer_with_format(&mut buffer, &camera_bytes, size.width as u32, size.height as u32, image::ColorType::Rgb8, ImageOutputFormat::Jpeg(70)).unwrap();

                                        //Send image
                                        voip_clone.send_image(uuid_clone.clone(), &buffer.into_inner().unwrap().into_inner(), &decryption_key_clone).await.unwrap();
                                    },
                                    None => {
                                        // . . .
                                    },
                                }
                            }
                            _ = cancel_token_clone.cancelled() => {
                                //Exit thread
                                break;
                            },
                        }
                    }
                });

                let enable_microphone = voip.enable_microphone.clone();

                //Sender thread
                tokio::spawn(async move {
                    //This variable is notifed when the Mutex is set to true, when the audio_buffer lenght reaches ```VOIP_PACKET_BUFFER_LENGHT``` and is resetted when the packet is sent
                    let voip_audio_buffer: Arc<std::sync::Mutex<VecDeque<f32>>> = Arc::new(std::sync::Mutex::new(VecDeque::new()));

                    //Conect socket to destination
                    voip.socket.connect(destination).await.unwrap();

                    //Start audio recorder
                    let recording_handle = record_audio_with_interrupt(rx, *microphone_precentage.lock().unwrap(), voip_audio_buffer.clone(), enable_microphone.clone()).unwrap();

                    //We can just send it becasue we have already set the default destination address
                    loop {
                        select! {
                            //Wait until we should send the buffer
                            //Record 35ms of audio, send it to the server
                            _ = tokio::time::sleep(Duration::from_millis(VOIP_PACKET_BUFFER_LENGHT_MS as u64)) => {
                                    //We create this scope to tell the compiler the recording handle wont be sent across any awaits
                                    let playbackable_audio: Vec<u8> = {
                                        //Lock handle
                                        let mut recording_handle = recording_handle.lock().unwrap();
                                        //Create wav bytes
                                        let playbackable_audio: Vec<u8> = create_wav_file(
                                            recording_handle.clone().into()
                                        );
                                        //Clear out buffer, make the capacity remain (We creted this VecDeque with said default capacity)
                                        recording_handle.clear();
                                        //Return wav bytes
                                        playbackable_audio
                                    };
                                    //Create audio chunks
                                    let audio_chunks = playbackable_audio.chunks(30000);
                                    //Avoid sending too much data (If there is more recorded we just iterate over the chunks and not send them at once)
                                    for chunk in audio_chunks {
                                        voip.send_audio(uuid.clone(), chunk.to_vec(), &decryption_key).await.unwrap();
                                    }
                            },
                            _ = cancel_token.cancelled() => {
                                //Exit thread
                                break;
                            },
                        };
                    }
                });

                //Clone ctx
                let ctx = ctx.clone();

                //Create sink
                let sink = Arc::new(rodio::Sink::try_new(&self.client_ui.audio_playback.stream_handle).unwrap());
                let decryption_key = self.client_connection.client_secret.clone();
                //Reciver thread
                tokio::spawn(async move {
                    let ctx_clone = ctx.clone();

                    //Create image buffer
                    let image_buffer: MessageBuffer = Arc::new(DashMap::new());

                    //Listen on socket, play audio
                    loop {
                        select! {
                            _ = cancel_token_child.cancelled() => {
                                //Break out of the listener loop
                                break;
                            },

                            //Recive bytes
                            _recived_bytes_count = async {
                                match recive_server_relay(reciver_socket_part.clone(), &decryption_key, sink.clone(), image_buffer.clone(), &ctx_clone).await {
                                    Ok(_) => (),
                                    Err(err) => {
                                        tracing::error!("{}", err);
                                    },
                                }
                            } => {}
                        }
                    }
                });
            });
        }
    }
}

/// Recives packets on the given UdpSocket, messages are decrypted with the decrpytion key
/// Automaticly appends the decrypted audio bytes to the ```Sink``` if its an uadio packet
/// I might rework this function so that we can see whos talking based on uuid
async fn recive_server_relay(
    //Socket this function is Listening on
    reciver_socket_part: Arc<tokio::net::UdpSocket>,
    //Decryption key
    decryption_key: &[u8],
    //The sink its appending the bytes to
    sink: Arc<Sink>,
    //This serves as the image buffer from the server
    image_buffer: MessageBuffer,

    ctx: &egui::Context,
) -> anyhow::Result<()>
{
    //Create buffer for header, this is the size of the maximum udp packet so no error will appear
    let mut header_buf = vec![0; 65536];

    //Recive header size
    reciver_socket_part
        .peek_from(&mut header_buf)
        .await
        .unwrap();

    //Get message lenght
    let header_lenght = u32::from_be_bytes(header_buf[..4].try_into().unwrap());

    //Create body according to message size indicated by the header, make sure to add 4 to the byte lenght because we peeked the ehader thus we didnt remove the bytes from the buffer
    let mut body_buf = vec![0; (header_lenght + 4) as usize];

    //Recive the whole message
    reciver_socket_part.recv(&mut body_buf).await.unwrap();

    //Decrypt message
    let mut decrypted_bytes = decrypt_aes256_bytes(
        //Only take the bytes from the 4th byte because thats the header
        &body_buf[4..],
        decryption_key,
    )?;

    let message_flag_bytes: Vec<u8> = decrypted_bytes.drain(decrypted_bytes.len() - 4..).collect();

    match UdpMessageType::from_number(u32::from_be_bytes(message_flag_bytes.try_into().unwrap())) {
        UdpMessageType::Voice => {
            //The generated uuids are always a set amount of bytes, so we can safely extract them, and we know that the the left over bytes are audio
            let uuid = String::from_utf8(
                decrypted_bytes
                    .drain(decrypted_bytes.len() - 36..)
                    .collect(),
            )?;

            //Make sure to verify that the UUID we are parsing is really a uuid, because if its not we know we have parsed the bytes in an incorrect order
            uuid::Uuid::parse_str(&uuid)
                .map_err(|err| anyhow::Error::msg(format!("Error: {}, in uuid {}", err, uuid)))?;

            //Play recived bytes
            sink.append(rodio::Decoder::new(BufReader::new(Cursor::new(
                decrypted_bytes,
            )))?);
        },
        UdpMessageType::ImageHeader => {
            get_image_header(&decrypted_bytes, &image_buffer).unwrap();
        },
        UdpMessageType::Image => {
            // [. . . . . . . . . . . len - 164][len - 164 . . . . . len - 100][len - 100. . . . . len - 64][len - 64 . . . .]
            //      IMAGE                           HASH                            UUID                      IDENTIFICATOR
            let message_bytes = decrypted_bytes.to_vec();

            //Get the identificator of the image part in bytes
            let indetificator_bytes = message_bytes[message_bytes.len() - 64..].to_vec();

            let identificator = String::from_utf8(indetificator_bytes).unwrap();

            //Get the identificator of the image part in bytes
            let hash_bytes = message_bytes
                [message_bytes.len() - 64 - 64 - 36..message_bytes.len() - 64 - 36]
                .to_vec();

            let hash = String::from_utf8(hash_bytes).unwrap();

            //Get the image part bytes
            //We subtract 164 bytes to only get the image part
            let image = message_bytes[..message_bytes.len() - 64 - 64 - 36].to_vec();

            let uuid = String::from_utf8(
                message_bytes[message_bytes.len() - 64 - 36..message_bytes.len() - 64].to_vec(),
            )
            .unwrap();

            //Make sure to verify that the UUID we are parsing is really a uuid, because if its not we know we have parsed the bytes in an incorrect order
            uuid::Uuid::parse_str(uuid.trim())
                .map_err(|err| anyhow::Error::msg(format!("Error: {}, in uuid {}", err, uuid)))?;

            if let Some(mut image_header) = image_buffer.get_mut(&uuid) {
                if let Some((index, _, contents)) = image_header.get_full_mut(&identificator) {
                    if let Some(byte_pair) = contents.get_mut(&hash) {
                        *byte_pair = Some(image);
                    }
                    else {
                        tracing::error!("Image part hash not found in the image header: {hash}");
                    }

                    //If all the parts of the image header had arrived send the image to all the clients
                    if contents.iter().all(|(_, value)| value.is_some()) {
                        let contents_clone = contents.clone();

                        //Combine the image part bytes
                        let image_bytes: Vec<u8> = contents_clone
                            .iter()
                            .flat_map(|(_, value)| {
                                <std::option::Option<std::vec::Vec<u8>> as Clone>::clone(value)
                                    .unwrap()
                            })
                            .collect();

                        //Define uri
                        let uri = format!("bytes://video_steam:{uuid}");

                        //If the image bytes are empty that means the video stream has shut down
                        if image_bytes == vec![0] {
                            //Forget image on that URI
                            ctx.forget_image(&uri);

                            image_buffer.remove(&uuid);
                        }
                        //Else save the image
                        else {
                            //Forget image on that URI
                            ctx.forget_image(&uri);

                            //Pair URI with bytes
                            ctx.include_bytes(uri, image_bytes);
                        }

                        //Request repaint
                        ctx.request_repaint();

                        //Drain earlier ImageHeaders (and the current one), because a new one has arrived
                        image_header.drain(index..=index);
                    }
                }
                else {
                    tracing::error!("Image header not found: {identificator}");
                }
            }
            else {
                tracing::error!("User not found in the image header list: {uuid}");
            }
        },
    }

    Ok(())
}
